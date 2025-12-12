use crate::order::*;
use derive_more::Constructor;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use derive_getters::Getters;

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Constructor, Getters)]
pub struct RequestOpen {
    #[getter(copy)]
    side: Side,
    #[getter(copy)]
    price: Decimal,
    #[getter(copy)]
    quantity: Decimal,
    #[getter(copy)]
    #[serde(rename = "type")]
    kind: OrderKind,
    #[getter(copy)]
    #[serde(rename = "newClientOrderId")]
    client_order_id: Uuid,
    #[getter(copy)]
    #[serde(rename = "timeInForce")]
    time_in_force: TimeInForce,
    #[getter(copy)]
    #[serde(rename = "goodTillDate", skip_serializing_if = "Option::is_none")]
    good_till_date: Option<u64>,
}

// #[derive(
//     Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize
// )]
// pub struct RequestCancel {
//     pub id: Option<ClientId>,
// }
