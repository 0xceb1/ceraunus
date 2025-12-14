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
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "o")]
    update: OrderTradeUpdate,
}

impl OrderTradeUpdateEvent {
    pub fn event_time(&self) -> DateTime<Utc> {
        self.event_time
    }

    pub fn transaction_time(&self) -> DateTime<Utc> {
        self.transaction_time
    }

    pub fn update(&self) -> &OrderTradeUpdate {
        &self.update
    }

    pub fn symbol(&self) -> Symbol {
        self.update.symbol
    }

    pub fn client_order_id(&self) -> Uuid {
        self.update.client_order_id
    }

    pub fn side(&self) -> Side {
        self.update.side
    }

    pub fn order_kind(&self) -> OrderKind {
        self.update.order_kind
    }

    pub fn time_in_force(&self) -> TimeInForce {
        self.update.time_in_force
    }

    pub fn orig_qty(&self) -> Decimal {
        self.update.orig_qty
    }

    pub fn orig_price(&self) -> Decimal {
        self.update.orig_price
    }

    pub fn avg_price(&self) -> Decimal {
        self.update.avg_price
    }

    pub fn exec_type(&self) -> ExecutionType {
        self.update.exec_type
    }

    pub fn order_status(&self) -> OrderStatus {
        self.update.order_status
    }

    pub fn order_id(&self) -> u64 {
        self.update.order_id
    }

    pub fn last_filled_qty(&self) -> Decimal {
        self.update.last_filled_qty
    }

    pub fn filled_qty(&self) -> Decimal {
        self.update.filled_qty
    }

    pub fn last_filled_price(&self) -> Decimal {
        self.update.last_filled_price
    }

    pub fn commission(&self) -> Decimal {
        self.update.commission
    }

    pub fn trade_time(&self) -> DateTime<Utc> {
        self.update.trade_time
    }

    pub fn trade_id(&self) -> u64 {
        self.update.trade_id
    }

    pub fn is_maker(&self) -> bool {
        self.update.is_maker
    }

    pub fn realized_profit(&self) -> Decimal {
        self.update.realized_profit
    }
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
#[derive(Debug, Clone, Copy, Deserialize)]
#[allow(dead_code)]
pub struct TradeLite {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: Symbol,
    #[serde(rename = "q")]
    orig_qty: Decimal,
    #[serde(rename = "p")]
    orig_price: Decimal,
    #[serde(rename = "m")]
    is_makter: bool,
    #[serde(rename = "c")]
    client_order_id: Uuid,
    #[serde(rename = "S")]
    side: Side,
    #[serde(rename = "L")]
    last_filled_price: Decimal,
    #[serde(rename = "l")]
    last_filled_qty: Decimal,

    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "i")]
    order_id: u64,
}