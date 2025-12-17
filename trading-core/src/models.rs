use chrono::{DateTime, Utc};
use data::binance::account::OrderTradeUpdateEvent;
use data::binance::market::Depth;
use data::binance::request::RequestOpen;
use data::order::*;
use derive_getters::Getters;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt::{self, Formatter};
use tracing::warn;
use uuid::Uuid;

use crate::error::Result as TradingCoreResult;

/// Local record for an order
#[derive(Debug, Clone, Copy, Getters)]
pub struct Order {
    symbol: Symbol,
    side: Side,
    start_ts: DateTime<Utc>,
    order_id: Option<u64>,
    #[getter(copy)]
    client_order_id: Uuid,
    last_update_ts: DateTime<Utc>,

    kind: OrderKind, // a limit order can be transformed into market order due to price drift
    curr_price: Decimal,
    curr_qty: Decimal,
    orig_price: Decimal,
    orig_qty: Decimal,
    time_in_force: TimeInForce,
    good_till_date: Option<u64>,
    status: Option<OrderStatus>,
}

impl Order {
    pub fn new(
        symbol: Symbol,
        side: Side,
        kind: OrderKind,
        price: Decimal,
        quantity: Decimal,
        time_in_force: TimeInForce,
        good_till_date: Option<u64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            symbol,
            side,
            start_ts: now,
            order_id: None,
            client_order_id: Uuid::new_v4(),
            last_update_ts: now,
            kind,
            curr_price: price,
            curr_qty: quantity,
            orig_price: price,
            orig_qty: quantity,
            time_in_force,
            good_till_date,
            status: None,
        }
    }

    pub fn to_request(&self) -> RequestOpen {
        RequestOpen::new(
            self.symbol,
            self.side,
            self.orig_price,
            self.orig_qty,
            self.kind,
            self.client_order_id,
            self.time_in_force,
            self.good_till_date,
        )
    }

    pub fn on_update_received(&mut self, update_event: &OrderTradeUpdateEvent) {
        // TODO: what timestamp is best here?
        self.last_update_ts = update_event.transaction_time();
        self.order_id = Some(update_event.order_id());
        self.status = Some(update_event.order_status());
        self.curr_price = update_event.last_filled_price();
        self.curr_qty = update_event.last_filled_qty();
        if update_event.order_kind() == OrderKind::Market && self.kind == OrderKind::Limit {
            warn!(
                client_id = %update_event.client_order_id(),
                order_status = %update_event.order_status(),
                total_filled_qty = %update_event.filled_qty(),
                this_filled_qty = %update_event.last_filled_qty(),
                this_filled_price =  %update_event.last_filled_price(),
                "A limit order is traded as market order"
            );
        }
        self.kind = update_event.order_kind();
    }
}

type Price = Decimal;
type Quantity = Decimal;
#[derive(Debug, Getters)]
pub struct OrderBook {
    symbol: Symbol,
    local_ts: DateTime<Utc>,
    xchg_ts: DateTime<Utc>,
    last_update_id: u64,
    bids: BTreeMap<Price, Quantity>,
    asks: BTreeMap<Price, Quantity>,
}

impl OrderBook {
    pub fn new(symbol: Symbol) -> Self {
        OrderBook {
            symbol,
            local_ts: Utc::now(),
            xchg_ts: Utc::now(),
            last_update_id: 0, // this is the id for the depth update
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub async fn from_snapshot(
        symbol: Symbol,
        depth: u16,
        endpoint: &str,
        client: Client,
    ) -> TradingCoreResult<Self> {
        let url = format!("{endpoint}/fapi/v1/depth?symbol={symbol}&limit={depth}");
        let response = client.get(url).send().await?;

        response.error_for_status_ref()?;
        let snapshot = response.json::<DepthSnapshot>().await?;
        Ok(OrderBook {
            symbol,
            local_ts: Utc::now(),
            last_update_id: snapshot.last_update_id,
            xchg_ts: snapshot.xchg_ts,
            bids: snapshot.bids,
            asks: snapshot.asks,
        })
    }

    pub fn show(&self, depth: usize) -> String {
        //TODO: benchmark the perf
        format!(
            "[B:{}|A:{}]",
            self.bids.iter().rev().take(depth)
                .map(|(p, q)| format!("{}@{}", q, p))
                .collect::<Vec<_>>()
                .join(","),
            self.asks.iter().take(depth)
                .map(|(p, q)| format!("{}@{}", q, p))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    pub fn extend(&mut self, depth: Depth) {
        // WARN: This is a dumb method, please check the last_update_id by yourself
        self.xchg_ts = depth.transaction_time();
        self.local_ts = Utc::now();
        self.last_update_id = depth.final_update_id();

        for level in depth.bids() {
            if level.quantity.is_zero() {
                self.bids.remove(&level.price);
            } else {
                self.bids.insert(level.price, level.quantity);
            }
        }

        for level in depth.asks() {
            if level.quantity.is_zero() {
                self.asks.remove(&level.price);
            } else {
                self.asks.insert(level.price, level.quantity);
            }
        }
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} OrderBook (last_update_id: {})",
            self.symbol, self.last_update_id
        )
    }
}

/// Helper struct to construct OrderBook from binance snapshot
#[derive(Deserialize)]
struct DepthSnapshot {
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    xchg_ts: DateTime<Utc>,
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    #[serde(deserialize_with = "de_side")]
    bids: BTreeMap<Price, Quantity>,
    #[serde(deserialize_with = "de_side")]
    asks: BTreeMap<Price, Quantity>,
}

/// Deserialize arrays of [price, qty] to BtreeMap.
fn de_side<'de, D>(deserializer: D) -> std::result::Result<BTreeMap<Price, Quantity>, D::Error>
where
    D: Deserializer<'de>,
{
    // TODO: Using CursorMut API
    // https://users.rust-lang.org/t/deserialising-my-btreemap-with-serde/110328
    // https://github.com/rust-lang/rust/issues/107540
    // Binance depth returns [["price", "qty"], ...]; let serde parse strings into Decimal.
    let raw: Vec<(Price, Quantity)> = Deserialize::deserialize(deserializer)?;
    let mut side = BTreeMap::new();
    side.extend(raw); // O(N*log(N))
    Ok(side)
}
