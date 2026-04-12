//! Payload models for Binance *market* data streams (regular market data)
use crate::types::Symbol;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Payload model for aggTrade stream
/// <https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Aggregate-Trade-Streams>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTrade {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "a")]
    agg_trade_id: u64,

    #[serde(rename = "p")]
    price: Decimal,

    #[serde(rename = "q")]
    quantity: Decimal,

    #[serde(rename = "f")]
    first_trade_id: u64,
    #[serde(rename = "l")]
    last_trade_id: u64,
    #[serde(rename = "m")]
    is_maker: bool,
}

/// Payload model for trade stream
/// Unfortunately, the trade stream only appears in Binance spot api docs
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Trade {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "t")]
    trade_id: u64,

    #[serde(rename = "p")]
    price: Decimal,

    #[serde(rename = "q")]
    quantity: Decimal,

    #[serde(rename = "m")]
    is_maker: bool,
}
