//! Payload models for Binance *public* data streams (high-frequency public market data)
use crate::types::{Level, Symbol};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

type OrderBookUpdateId = u64;

/// Payload model for depth update stream, either snapshot or incremental update
/// <https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Diff-Book-Depth-Streams>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Depth {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    pub transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "U")]
    pub first_update_id: OrderBookUpdateId,

    #[serde(rename = "u")]
    pub final_update_id: OrderBookUpdateId,

    #[serde(rename = "pu")]
    pub last_final_update_id: OrderBookUpdateId,

    #[serde(rename = "b")]
    pub bids: Vec<Level>,
    #[serde(rename = "a")]
    pub asks: Vec<Level>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BookTicker {
    #[serde(rename = "u")]
    order_book_update_id: OrderBookUpdateId,

    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "b")]
    pub bid_price: Decimal,

    #[serde(rename = "B")]
    pub bid_qty: Decimal,

    #[serde(rename = "a")]
    pub ask_price: Decimal,

    #[serde(rename = "A")]
    pub ask_qty: Decimal,
}
