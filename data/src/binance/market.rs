use crate::order::Symbol;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

type OrderBookUpdateId = u64;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(from = "(Decimal, Decimal)")]
pub struct Level {
    pub price: Decimal,
    pub quantity: Decimal,
}

impl From<(Decimal, Decimal)> for Level {
    fn from((price, quantity): (Decimal, Decimal)) -> Self {
        Self { price, quantity }
    }
}

impl From<(&Decimal, &Decimal)> for Level {
    fn from((price, quantity): (&Decimal, &Decimal)) -> Self {
        Self {
            price: *price,
            quantity: *quantity,
        }
    }
}

impl From<Level> for (Decimal, Decimal) {
    fn from(level: Level) -> Self {
        (level.price, level.quantity)
    }
}

/// Payload model for depth update stream, either snapshot or incremental update
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Mark-Price-Stream
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct Depth {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,

    #[serde(rename = "U")]
    #[getter(copy)]
    first_update_id: OrderBookUpdateId,

    #[serde(rename = "u")]
    #[getter(copy)]
    final_update_id: OrderBookUpdateId,

    #[serde(rename = "pu")]
    #[getter(copy)]
    last_final_update_id: OrderBookUpdateId,

    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}

/// Payload model for aggTrade stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Aggregate-Trade-Streams
#[derive(Debug, Clone, Serialize, Deserialize, Getters)]
pub struct AggTrade {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,

    #[serde(rename = "a")]
    agg_trade_id: u64,

    #[serde(rename = "p")]
    #[getter(copy)]
    price: Decimal,

    #[serde(rename = "q")]
    #[getter(copy)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Getters)]
pub struct Trade {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,

    #[serde(rename = "t")]
    trade_id: u64,

    #[serde(rename = "p")]
    #[getter(copy)]
    price: Decimal,

    #[serde(rename = "q")]
    #[getter(copy)]
    quantity: Decimal,

    #[serde(rename = "m")]
    is_maker: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Getters)]
pub struct BookTicker {
    #[serde(rename = "u")]
    #[getter(copy)]
    order_book_update_id: OrderBookUpdateId,

    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,

    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,

    #[serde(rename = "b")]
    #[getter(copy)]
    bid_price: Decimal,

    #[serde(rename = "B")]
    #[getter(copy)]
    bid_qty: Decimal,

    #[serde(rename = "a")]
    #[getter(copy)]
    ask_price: Decimal,

    #[serde(rename = "A")]
    #[getter(copy)]
    ask_qty: Decimal,
}
