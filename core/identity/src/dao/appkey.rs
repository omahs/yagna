pub use crate::dao::Error as DaoError;
pub use crate::db::models::{AppKey, Role};
use chrono::Local;
use diesel::prelude::*;

use diesel::{Connection, ExpressionMethods, RunQueryDsl};
use std::cmp::max;
use ya_core_model::ethaddr::NodeId;
use ya_persistence::executor::{do_with_connection, AsDao, ConnType, PoolType};

pub type Result<T> = std::result::Result<T, DaoError>;

pub struct AppKeyDao<'c> {
    pool: &'c PoolType,
}

impl<'a> AsDao<'a> for AppKeyDao<'a> {
    fn as_dao(pool: &'a PoolType) -> Self {
        AppKeyDao { pool }
    }
}

impl<'c> AppKeyDao<'c> {
    pub async fn with_connection<R: Send + 'static, F>(&self, f: F) -> Result<R>
    where
        F: Send + 'static + FnOnce(&ConnType) -> Result<R>,
    {
        do_with_connection(&self.pool, f).await
    }

    #[inline]
    async fn with_transaction<
        R: Send + 'static,
        F: FnOnce(&ConnType) -> Result<R> + Send + 'static,
    >(
        &self,
        f: F,
    ) -> Result<R> {
        self.with_connection(move |conn| conn.transaction(|| f(conn)))
            .await
    }

    pub async fn create(
        &self,
        key: String,
        name: String,
        role: String,
        identity: NodeId,
    ) -> Result<()> {
        use crate::db::schema::app_key as app_key_dsl;
        use crate::db::schema::role as role_dsl;

        do_with_connection(self.pool, move |conn| {
            conn.transaction(|| {
                let role: Role = role_dsl::table
                    .filter(role_dsl::name.eq(role))
                    .first(conn)?;

                diesel::insert_into(app_key_dsl::table)
                    .values((
                        app_key_dsl::role_id.eq(&role.id),
                        app_key_dsl::name.eq(name),
                        app_key_dsl::key.eq(key),
                        app_key_dsl::identity_id.eq(identity),
                        app_key_dsl::created_date.eq(Local::now().naive_local()),
                    ))
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
    }

    pub async fn get(&self, key: String) -> Result<(AppKey, Role)> {
        use crate::db::schema::app_key as app_key_dsl;
        use crate::db::schema::role as role_dsl;

        self.with_transaction(|conn| {
            let result = app_key_dsl::table
                .inner_join(role_dsl::table)
                .filter(app_key_dsl::key.eq(key))
                .first(conn)?;

            Ok(result)
        })
        .await
    }

    pub async fn list(
        &self,
        identity: Option<String>,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<(AppKey, Role)>, u32)> {
        use crate::db::schema::app_key as app_key_dsl;
        use crate::db::schema::role as role_dsl;

        let offset = max(0, (page - 1) * per_page);
        self.with_transaction(move |conn| {
            let query = app_key_dsl::table
                .inner_join(role_dsl::table)
                .limit(per_page as i64)
                .offset(offset as i64);

            let results: Vec<(AppKey, Role)> = if let Some(id) = identity {
                query.filter(app_key_dsl::identity_id.eq(id)).load(conn)
            } else {
                query.load(conn)
            }?;

            // TODO: use DB INSERT / DELETE triggers and internal counters in place of count
            let total: i64 = app_key_dsl::table
                .select(diesel::expression::dsl::count(app_key_dsl::id))
                .first(conn)?;
            let pages = (total as f64 / per_page as f64).ceil() as u32;

            Ok((results, pages))
        })
        .await
    }

    pub async fn remove(&self, name: String, identity: Option<String>) -> Result<()> {
        use crate::db::schema::app_key as app_key_dsl;

        self.with_transaction(move |conn| {
            let filter = app_key_dsl::table.filter(app_key_dsl::name.eq(name.as_str()));
            if let Some(id) = identity {
                diesel::delete(filter.filter(app_key_dsl::identity_id.eq(id.as_str())))
                    .execute(conn)
            } else {
                diesel::delete(filter).execute(conn)
            }?;

            Ok(())
        })
        .await
    }
}
