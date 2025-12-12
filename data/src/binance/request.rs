use crate::order::*;
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor,
)]
pub struct RequestOpen {
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    #[serde(rename = "type")]
    pub kind: OrderKind,
    #[serde(rename = "newClientOrderId")]
    pub client_order_id: Uuid,
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "goodTillDate", skip_serializing_if = "Option::is_none")]
    pub good_till_date: Option<u64>,
}

// #[derive(
//     Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize
// )]
// pub struct RequestCancel {
//     pub id: Option<ClientId>,
// }
