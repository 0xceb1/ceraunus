use crate::order::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExecutionType {
    New,
    Canceled,
    Calculated,
    Expired,
    Trade,
    Amendment,
}

/// Top-level payload model for verbose `ORDER_TRADE_UPDATE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Order-Update
#[derive(Debug, Deserialize)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    pub event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    pub transaction_time: DateTime<Utc>,
    #[serde(rename = "o")]
    pub order: OrderTradeUpdate,
}

#[derive(Debug, Deserialize)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    pub symbol: Symbol,
    #[serde(rename = "c")]
    pub client_order_id: Uuid,
    #[serde(rename = "S")]
    pub side: Side,
    #[serde(rename = "o")]
    pub order_type: OrderKind,
    #[serde(rename = "f")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "q")]
    pub orig_qty: Decimal,
    #[serde(rename = "p")]
    pub orig_price: Decimal,
    #[serde(rename = "ap")]
    pub avg_price: Decimal,
    #[serde(rename = "x")]
    pub exec_type: ExecutionType,
    #[serde(rename = "X")]
    pub order_status: OrderStatus,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "l")]
    pub last_filled_qty: Decimal,
    #[serde(rename = "z")]
    pub filled_qty: Decimal,
    #[serde(rename = "L")]
    pub last_filled_price: Decimal,
    #[serde(rename = "n")]
    pub commission: Decimal,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    pub trade_time: DateTime<Utc>,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "rp")]
    pub realized_profit: Decimal,
}

/// Payload model for `TRADE_LITE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Trade-Lite
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TradeLite {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    pub event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    pub transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    pub symbol: Symbol,
    #[serde(rename = "q")]
    pub orig_qty: Decimal,
    #[serde(rename = "p")]
    pub orig_price: Decimal,
    #[serde(rename = "m")]
    pub is_makter: bool,
    #[serde(rename = "c")]
    pub client_order_id: Uuid,
    #[serde(rename = "S")]
    pub side: Side,
    #[serde(rename = "L")]
    pub last_filled_price: Decimal,
    #[serde(rename = "l")]
    pub last_filled_qty: Decimal,

    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "i")]
    pub order_id: u64,
}
