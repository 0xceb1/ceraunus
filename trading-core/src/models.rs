use chrono::{DateTime, Utc};
use data::binance::account::OrderTradeUpdateEvent;
use data::binance::market::{Depth, Level};
use data::order::*;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use tracing::warn;
use uuid::Uuid;

use crate::error::Result as TradingCoreResult;

type BboPair = (Level, Level); // (bid_level, ask_level)

/// Local record for an order
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Order {
    symbol: Symbol,
    side: Side,
    #[serde(skip)]
    #[allow(dead_code)]
    start_ts: DateTime<Utc>,
    #[serde(skip_serializing)]
    order_id: Option<u64>,
    #[serde(rename = "newClientOrderId")]
    pub client_order_id: Uuid,
    #[serde(skip_serializing)]
    pub last_update_ts: DateTime<Utc>,

    #[serde(rename = "type")]
    kind: OrderKind, // a limit order can be transformed into market order due to price drift
    #[serde(skip_serializing)]
    curr_price: Decimal,
    #[serde(skip_serializing)]
    curr_qty: Decimal,
    #[serde(rename = "price")]
    orig_price: Decimal,
    #[serde(rename = "quantity")]
    orig_qty: Decimal,
    #[serde(rename = "timeInForce")]
    pub time_in_force: TimeInForce,
    #[serde(rename = "goodTillDate", skip_serializing_if = "Option::is_none")]
    pub good_till_date: Option<u64>,
    #[serde(skip_serializing)]
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

#[derive(Debug)]
pub struct OrderBook {
    symbol: Symbol,
    local_ts: DateTime<Utc>,
    xchg_ts: DateTime<Utc>,
    pub last_update_id: u64,
    pub bids: Vec<Level>, // price low to high
    pub asks: Vec<Level>, // price high to low
}

impl OrderBook {
    pub fn new(symbol: Symbol) -> Self {
        OrderBook {
            symbol,
            local_ts: Utc::now(),
            xchg_ts: Utc::now(),
            last_update_id: 0, // this is the id for the depth update
            bids: Vec::with_capacity(128),
            asks: Vec::with_capacity(128),
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
            // TODO: reverse when parsing?
            bids: snapshot.bids.into_iter().rev().collect(),
            asks: snapshot.asks.into_iter().rev().collect(),
        })
    }

    pub fn show(&self, depth: usize) -> String {
        format!(
            "[B:{}|A:{}]",
            self.bids
                .iter()
                .rev()
                .take(depth)
                .map(|Level { price, quantity }| format!("{quantity}@{price}"))
                .collect::<Vec<_>>()
                .join(","),
            self.asks
                .iter()
                .rev()
                .take(depth)
                .map(|Level { price, quantity }| format!("{quantity}@{price}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    /// Neither self.bids nor self.asks should be empty
    pub fn extend(&mut self, depth: Depth) {
        self.xchg_ts = depth.transaction_time;
        self.local_ts = Utc::now();
        self.last_update_id = depth.final_update_id;

        for upd in &depth.bids {
            let mut i = self.bids.len();
            loop {
                if i == 0 {
                    self.bids.insert(0, *upd);
                    break;
                }
                i -= 1;
                if self.bids[i].price == upd.price {
                    if upd.quantity.is_zero() {
                        self.bids.remove(i);
                    } else {
                        self.bids[i].quantity = upd.quantity;
                    }
                    break;
                } else if self.bids[i].price < upd.price {
                    self.bids.insert(i + 1, *upd);
                    break;
                }
            }
        }

        for upd in &depth.asks {
            let mut i = self.asks.len();
            loop {
                if i == 0 {
                    self.asks.insert(0, *upd);
                    break;
                }
                i -= 1;
                if self.asks[i].price == upd.price {
                    if upd.quantity.is_zero() {
                        self.asks.remove(i);
                    } else {
                        self.asks[i].quantity = upd.quantity;
                    }
                    break;
                } else if self.asks[i].price > upd.price {
                    self.asks.insert(i + 1, *upd);
                    break;
                }
            }
        }
    }

    pub fn get_bbo(&self) -> Option<BboPair> {
        let Level {
            price: bp,
            quantity: bq,
        } = self.bids.last()?;
        let Level {
            price: ap,
            quantity: aq,
        } = self.asks.last()?;
        Some((Level::from((bp, bq)), Level::from((ap, aq))))
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
    bids: Vec<Level>,
    asks: Vec<Level>,
}

/// PnL per symbol
#[derive(Debug, Clone, Copy)]
pub struct ProfitAndLoss {
    pub execution_pnl: Decimal, // WARN: in USDT, Commission??
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    avg_entry_price: Decimal,
    pub position: Decimal, // as of qty
    buy_qty: Decimal,
    sell_qty: Decimal,
    buy_amount: Decimal,
    sell_amount: Decimal,
}

impl ProfitAndLoss {
    pub fn new(init_price: Decimal, init_pos: Decimal) -> Self {
        const ZERO: Decimal = Decimal::ZERO;
        Self {
            execution_pnl: ZERO,
            unrealized_pnl: ZERO,
            realized_pnl: ZERO,
            avg_entry_price: init_price,
            position: init_pos,
            buy_qty: ZERO,
            sell_qty: ZERO,
            buy_amount: ZERO,
            sell_amount: ZERO,
        }
    }

    pub fn on_update_received(&mut self, update_event: &OrderTradeUpdateEvent) {
        // TODO: benchmark the time usage
        // This method should only be called when trade event received
        self.execution_pnl -= update_event.commission();
        let price = update_event.last_filled_price();
        let qty = update_event.last_filled_qty();
        let amount = update_event.last_filled_amount();

        match update_event.side() {
            // handle realized pnl & position
            Side::Buy => self.handle_buy(price, qty, amount),
            Side::Sell => self.handle_sell(price, qty, amount),
        }

        // update unrealized pnl
        self.unrealized_pnl = (price - self.avg_entry_price) * self.position;
    }

    fn handle_buy(&mut self, price: Decimal, qty: Decimal, amount: Decimal) {
        let old_pos = self.position;
        self.position += qty;
        self.buy_qty += qty;
        self.buy_amount += amount;
        if old_pos >= Decimal::ZERO {
            let total_cost = self.avg_entry_price * old_pos + amount;
            self.avg_entry_price = total_cost / self.position;
        } else if qty <= -old_pos {
            self.realized_pnl += (self.avg_entry_price - price) * qty;
        } else {
            self.realized_pnl += (price - self.avg_entry_price) * old_pos;
            self.avg_entry_price = price;
        }
    }

    fn handle_sell(&mut self, price: Decimal, qty: Decimal, amount: Decimal) {
        let old_pos = self.position;
        self.position -= qty;
        self.sell_qty += qty;
        self.sell_amount += amount;

        if old_pos <= Decimal::ZERO {
            let total_cost = amount - self.avg_entry_price * self.position;
            self.avg_entry_price = -total_cost / old_pos;
        } else if qty <= old_pos {
            self.realized_pnl += (price - self.avg_entry_price) * qty;
        } else {
            self.realized_pnl += (price - self.avg_entry_price) * old_pos;
            self.avg_entry_price = price;
        }
    }
}
