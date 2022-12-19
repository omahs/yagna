use actix_cors::Cors;
use actix_web::dev::RequestHead;
use actix_web::http::header;
use actix_web::http::header::HeaderValue;
use actix_web_httpauth::headers::authorization::{Bearer, Scheme};

use anyhow::anyhow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use structopt::StructOpt;
use url::Url;

use crate::middleware::auth::resolver::AppKeyResolver;

use ya_core_model::appkey as model;
use ya_service_api_cache::AutoResolveCache;
use ya_service_bus::typed as bus;
use ya_service_bus::RpcEndpoint;

pub const BUS_ID: &str = "/local/middleware/cors";

pub type Cache = AutoResolveCache<AppKeyResolver>;

#[derive(Clone, StructOpt, Debug)]
pub struct CorsConfig {
    #[structopt(long)]
    allowed_origin: Url,
    /// Set a maximum time (in seconds) for which this CORS request may be cached.
    #[structopt(long, default_value = "3600")]
    max_age: usize,
}

#[derive(Clone)]
pub struct AppKeyCors {
    /// Holds AppKey and Allowed Origins pairs.
    cors: Arc<RwLock<HashMap<String, String>>>,
    config: Arc<CorsConfig>,
}

impl AppKeyCors {
    pub async fn new(config: &CorsConfig) -> anyhow::Result<AppKeyCors> {
        let appkey_cache = AppKeyCors {
            cors: Arc::new(Default::default()),
            config: Arc::new(config.clone()),
        };
        appkey_cache
            .listen_events()
            .await
            .map_err(|e| anyhow!("Can't build cors middleware: {e}"))?;
        Ok(appkey_cache)
    }

    pub fn cors(&self) -> Cors {
        let this = self.clone();
        let config = self.config.clone();

        Cors::default()
            .allowed_origin(&config.allowed_origin.to_string())
            .allowed_origin_fn(move |header, request| this.verify_origin(header, request))
            .allowed_methods(vec!["GET", "POST", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(config.max_age)
    }

    fn get(&self, key: &str) -> Option<String> {
        match self.cors.read() {
            Ok(cors) => cors.get(key).cloned(),
            Err(_) => None,
        }
    }

    fn update(&self, key: &str, origins: Option<String>) {
        if let Ok(mut cors) = self.cors.write() {
            match origins {
                None => cors.remove(key),
                Some(origins) => cors.insert(key.to_string(), origins.to_string()),
            };
        }
    }

    pub async fn listen_events(&self) -> anyhow::Result<()> {
        let this = self.clone();
        let endpoint = BUS_ID.to_string();

        let _ = bus::bind(&endpoint, move |event: model::event::Event| {
            let this = this.clone();
            async move {
                match event {
                    model::event::Event::NewKey(appkey) => this.update(&appkey.key, None),
                    model::event::Event::DroppedKey(appkey) => this.update(&appkey.key, None),
                };
                Ok(())
            }
        });
        bus::service(model::BUS_ID)
            .send(model::Subscribe { endpoint })
            .await??;
        Ok(())
    }

    fn verify_origin(&self, header: &HeaderValue, _request: &RequestHead) -> bool {
        let key = Bearer::parse(header).ok().map(|b| b.token().to_string());
        match key {
            Some(key) => match self.cors.read().unwrap().get(&key) {
                None => false,
                Some(_origins) => {
                    unimplemented!();
                }
            },
            None => false,
        }
    }
}
