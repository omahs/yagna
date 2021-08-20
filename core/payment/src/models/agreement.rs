use crate::schema::pay_agreement;
use crate::DEFAULT_PAYMENT_PLATFORM;
use chrono::{NaiveDateTime, Timelike, Utc};
use serde_json::Value;
use ya_agreement_utils::agreement::{expand, TypedPointer};
use ya_client_model::market::Agreement;
use ya_client_model::NodeId;
use ya_persistence::types::{BigDecimalField, Role};

#[derive(Queryable, Debug, Identifiable, Insertable)]
#[table_name = "pay_agreement"]
#[primary_key(id, owner_id)]
pub struct WriteObj {
    pub id: String,
    pub owner_id: NodeId,
    pub role: Role,
    pub peer_id: NodeId,
    pub payee_addr: String,
    pub payer_addr: String,
    pub payment_platform: String,
    pub total_amount_due: BigDecimalField,
    pub total_amount_accepted: BigDecimalField,
    pub total_amount_scheduled: BigDecimalField,
    pub total_amount_paid: BigDecimalField,
    pub app_session_id: Option<String>,
    pub created_ts: Option<NaiveDateTime>,
    pub updated_ts: Option<NaiveDateTime>,
}

impl WriteObj {
    pub fn new(agreement: Agreement, role: Role) -> Self {
        let provider_id = agreement.provider_id().clone();
        let requestor_id = agreement.requestor_id().clone();
        let (owner_id, peer_id) = match &role {
            Role::Provider => (provider_id.clone(), requestor_id.clone()),
            Role::Requestor => (requestor_id.clone(), provider_id.clone()),
        };

        let demand_properties = expand(agreement.demand.properties);
        let offer_properties = expand(agreement.offer.properties);

        let payment_platform = demand_properties
            .pointer("/golem/com/payment/chosen-platform")
            .as_typed(Value::as_str)
            .unwrap_or(DEFAULT_PAYMENT_PLATFORM)
            .to_owned();
        let payee_addr = offer_properties
            .pointer(format!("/golem/com/payment/platform/{}/address", payment_platform).as_str())
            .as_typed(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or(provider_id.to_string().to_lowercase());
        let payer_addr = demand_properties
            .pointer(format!("/golem/com/payment/platform/{}/address", payment_platform).as_str())
            .as_typed(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or(requestor_id.to_string().to_lowercase());

        let now = Utc::now();
        let created_ts = Some(now.naive_utc()).and_then(|v| v.with_nanosecond(0));
        let updated_ts = created_ts.clone();

        Self {
            id: agreement.agreement_id,
            owner_id,
            role,
            peer_id,
            payee_addr,
            payer_addr,
            payment_platform,
            total_amount_due: Default::default(),
            total_amount_accepted: Default::default(),
            total_amount_scheduled: Default::default(),
            total_amount_paid: Default::default(),
            app_session_id: agreement.app_session_id,
            created_ts,
            updated_ts,
        }
    }
}

pub type ReadObj = WriteObj;
