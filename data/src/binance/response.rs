use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::order::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenOrderSuccess {
    pub order_id: u64,
    symbol: Symbol,
    status: OrderStatus,
    pub client_order_id: Uuid,
    price: Decimal,        // quoted price
    avg_price: Decimal,    // avg filled price
    orig_qty: Decimal,     // initial quoted quantity
    executed_qty: Decimal, // filled quantity
    cum_qty: Decimal,      // filled quantity
    cum_quote: Decimal,    // filled amount in usdt
    side: Side,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    update_time: DateTime<Utc>,
}
