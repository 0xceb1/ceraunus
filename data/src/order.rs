use derive_more::Display;
use enum_map::Enum;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
pub enum OrderKind {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    ExpiredInMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Display)]
pub enum Asset {
    USDT,
    BUSD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Display, Enum)]
pub enum Symbol {
    BTCUSDT,
    ETHUSDT,
    SOLUSDT,
    BNBUSDT,
}

impl Symbol {
    pub fn as_str_lowercase(&self) -> &str {
        use Symbol as S;
        match self {
            S::BTCUSDT => "btcusdt",
            S::ETHUSDT => "ethusdt",
            S::SOLUSDT => "solusdt",
            S::BNBUSDT => "bnbusdt",
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, Display)]
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
