use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use crate::order::*;

#[derive(
    Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize
)]
pub struct RequestOpen {
    pub side: Side,
    pub price: Decimal,
    pub quantity: Decimal,
    pub kind: OrderKind,
    pub time_in_force: TimeInForce,
}

// #[derive(
//     Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Deserialize, Serialize
// )]
// pub struct RequestCancel {
//     pub id: Option<ClientId>,
// }