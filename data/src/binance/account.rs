use crate::order::*;
use chrono::{DateTime, Utc};
use derive_getters::Getters;
use derive_more::Display;
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
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
#[derive(Debug, Clone, Copy, Deserialize, Getters)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "o")]
    update: OrderTradeUpdate,
}

#[derive(Debug, Clone, Copy, Deserialize, Getters)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,

    #[serde(rename = "c")]
    #[getter(copy)]
    client_order_id: Uuid,

    #[serde(rename = "S")]
    #[getter(copy)]
    side: Side,

    #[serde(rename = "o")]
    #[getter(copy)]
    order_kind: OrderKind,

    #[serde(rename = "f")]
    #[getter(copy)]
    time_in_force: TimeInForce,

    #[serde(rename = "q")]
    #[getter(copy)]
    orig_qty: Decimal,

    #[serde(rename = "p")]
    #[getter(copy)]
    orig_price: Decimal,

    #[serde(rename = "ap")]
    #[getter(copy)]
    avg_price: Decimal,

    #[serde(rename = "x")]
    exec_type: ExecutionType,

    #[serde(rename = "X")]
    #[getter(copy)]
    order_status: OrderStatus,

    #[serde(rename = "i")]
    order_id: u64,

    #[serde(rename = "l")]
    #[getter(copy)]
    last_filled_qty: Decimal,

    #[serde(rename = "z")]
    #[getter(copy)]
    filled_qty: Decimal,

    #[serde(rename = "L")]
    #[getter(copy)]
    last_filled_price: Decimal,

    #[serde(rename = "n")]
    #[getter(copy)]
    commission: Decimal,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    trade_time: DateTime<Utc>,

    #[serde(rename = "t")]
    trade_id: u64,

    #[serde(rename = "m")]
    is_maker: bool,

    #[serde(rename = "rp")]
    #[getter(copy)]
    realized_profit: Decimal,
}

/// Payload model for `TRADE_LITE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Trade-Lite
#[derive(Debug, Clone, Copy, Deserialize, Getters)]
#[allow(dead_code)]
pub struct TradeLite {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    #[getter(copy)]
    symbol: Symbol,
    #[serde(rename = "q")]
    #[getter(copy)]
    orig_qty: Decimal,
    #[serde(rename = "p")]
    #[getter(copy)]
    orig_price: Decimal,
    #[serde(rename = "m")]
    is_makter: bool,
    #[serde(rename = "c")]
    #[getter(copy)]
    client_order_id: Uuid,
    #[serde(rename = "S")]
    #[getter(copy)]
    side: Side,
    #[serde(rename = "L")]
    #[getter(copy)]
    last_filled_price: Decimal,
    #[serde(rename = "l")]
    #[getter(copy)]
    last_filled_qty: Decimal,

    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "i")]
    order_id: u64,
}
