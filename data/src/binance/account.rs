use serde::Deserialize;
use chrono::{DateTime, Utc};
use crate::order::*;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag="x", rename_all="UPPERCASE")]
pub enum OrderTradeUpdate {
    New,
    Canceled,
    Calculated,
    Expired,
    Trade,
    Amendment
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TradeLite {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    pub event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    pub transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: Symbol,
    #[serde(rename = "q")]
    orig_qty: Decimal,    
    #[serde(rename = "p")]
    orig_price: Decimal,
    #[serde(rename = "m")]
    is_trader_market_makter: bool,
    #[serde(rename = "c")]
    client_order_id: Uuid,
    #[serde(rename = "S")]
    side : Side,
    #[serde(rename = "L")]
    last_filled_price: Decimal,
    #[serde(rename = "l")]
    last_filled_qty: Decimal,

    #[serde(rename = "t")]
    trade_id: u64, 
    #[serde(rename = "i")]
    order_id: u64,
}
