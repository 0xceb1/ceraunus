use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderKind {
    Limit,
    Market,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Display)]
pub enum Symbol {
    BTCUSDT,
    ETHUSDT,
    SOLUSDT,
    BNBUSDT,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize, Display,
)]
pub enum TimeInForce {
    #[serde(rename = "GTC")]
    GoodUntilCancel,
    #[serde(rename = "GTD")]
    GoodUntilDate { good_till_date: u64 },
    #[serde(rename = "GTX")]
    GoodTillCrossing,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
}
