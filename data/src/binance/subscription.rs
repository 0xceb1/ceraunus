use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

use crate::order::Symbol;

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

#[derive(Debug, Deserialize)]
pub struct StreamEnvelope<T> {
    pub stream: String,
    pub data: T,
}

/// Partial Book Depth Streams
/// https://developers.binance.com/docs/zh-CN/derivatives/usds-margined-futures/websocket-market-streams/Mark-Price-Stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDepth {
    #[serde(rename="E", with = "chrono::serde::ts_milliseconds")]
    event_time : DateTime<Utc>,
    #[serde(rename="T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename="s")]
    symbol: Symbol,
    #[serde(rename="U")]
    first_update_id: u64,
    #[serde(rename="u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}