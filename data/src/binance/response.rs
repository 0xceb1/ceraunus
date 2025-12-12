use chrono::{DateTime, Utc};
use derive_getters::Getters;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::order::*;

#[derive(Debug, Serialize, Deserialize, Getters)]
#[serde(rename_all = "camelCase")]
pub struct OrderSuccessResp {
    order_id: u64,
    #[getter(copy)]
    symbol: Symbol,
    #[getter(copy)]
    status: OrderStatus,
    #[getter(copy)]
    client_order_id: Uuid,
    #[getter(copy)]
    price: Decimal, // quoted price
    // avg_price: Decimal,    // avg filled price
    #[getter(copy)]
    orig_qty: Decimal, // initial quoted quantity
    #[getter(copy)]
    executed_qty: Decimal, // filled quantity
    #[getter(copy)]
    cum_qty: Decimal, // filled quantity
    #[getter(copy)]
    cum_quote: Decimal, // filled amount in usdt
    #[getter(copy)]
    side: Side,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    #[getter(copy)]
    update_time: DateTime<Utc>,
}
