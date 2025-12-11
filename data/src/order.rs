use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumString};
use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Display,
    AsRefStr,
    EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum OrderKind {
    Limit,
    Market,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    Display,
    AsRefStr,
    EnumString,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    Display,
    AsRefStr,
    EnumString,
)]
pub enum Symbol {
    BTCUSDT,
    ETHUSDT,
    SOLUSDT,
    BNBUSDT,
}

#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Deserialize,
    Serialize,
    Display,
    AsRefStr,
    EnumString,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
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
    GoodUntilDate,
    #[serde(rename = "GTX")]
    GoodTillCrossing,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
}
