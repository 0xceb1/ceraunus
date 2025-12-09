use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{AsRefStr, Display, EnumString};

use crate::order::Symbol;

#[derive(Debug, Deserialize)]
pub struct StreamEnvelope<T> {
    pub stream: String,
    pub data: T,
}

#[derive(Debug, Serialize, Clone, Display, AsRefStr, EnumString)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
enum WsRequestMethod {
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, Serialize)]
pub struct WsCommand {
    method: WsRequestMethod,
    params: Vec<String>, // stream names per Binance docs
    id: u64,
}

impl fmt::Display for WsCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl WsCommand {
    pub fn new(method: &str, params: Vec<String>, id: u64) -> Self {
        Self {
            method: method.parse().expect("Check your spell!"),
            params,
            id,
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(from = "(Decimal, Decimal)")]
pub struct Level {
    pub price: Decimal,
    pub amount: Decimal,
}

impl From<(Decimal, Decimal)> for Level {
    fn from((price, amount): (Decimal, Decimal)) -> Self {
        Self { price, amount }
    }
}

/// Partial Book Depth Streams
/// https://developers.binance.com/docs/zh-CN/derivatives/usds-margined-futures/websocket-market-streams/Mark-Price-Stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDepth {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: Symbol,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}
