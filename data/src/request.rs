use crate::order::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use derive_more::Constructor;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Constructor)]
pub struct RequestOpen {
    pub symbol : Symbol,
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    #[serde(rename="type")]
    pub kind: OrderKind,
    #[serde(rename="timeInForce")]
    pub time_in_force: TimeInForce,
}

// #[derive(
//     Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize
// )]
// pub struct RequestCancel {
//     pub id: Option<ClientId>,
// }
