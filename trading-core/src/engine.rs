use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use uuid::Uuid;

use crate::{
    error::{Result as TradingCoreResult, TradingCoreError},
    models::*,
};
use data::{
    binance::{
        account::OrderTradeUpdateEvent,
        market::{BookTicker, Level},
    },
    order::*,
};
use tracing::debug;

type BboPair = (Level, Level);

#[derive(Debug)]
pub struct State {
    pub symbol: Symbol,

    // best-available ask & bid
    pub bbo_level: Option<BboPair>, // (bid_level, ask_level)

    // local order book
    pub order_book: Option<OrderBook>,

    // orders that may still receive updates
    active_orders: FxHashMap<Uuid, Order>,

    // TODO: add a buffer for handling rejected orders

    // orders filled/cancelled/failed to sent (life ended)
    hist_orders: FxHashSet<Uuid>,

    pub pnl: ProfitAndLoss,

    start_time: DateTime<Utc>,

    // total traded amount in USDT
    // TODO: deprecate in the future
    turnover: Decimal,
}

impl State {
    pub fn new(symbol: Symbol) -> Self {
        Self {
            symbol,
            bbo_level: None,
            order_book: None,
            active_orders: FxHashMap::with_capacity_and_hasher(128, FxBuildHasher),
            hist_orders: FxHashSet::with_capacity_and_hasher(1024, FxBuildHasher),
            // TODO: construct from init pos
            pnl: ProfitAndLoss::new(Decimal::ZERO, Decimal::ZERO),
            start_time: Utc::now(),
            turnover: Decimal::ZERO,
        }
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    pub fn turnover(&self) -> Decimal {
        self.turnover
    }

    pub fn get_position(&self) -> Decimal {
        self.pnl.position()
    }

    // Order book management
    pub fn remove_order_book(&mut self) {
        self.order_book = None;
    }

    pub fn has_order_book(&self) -> bool {
        self.order_book.is_some()
    }

    // Active order tracking
    pub fn register_order(&mut self, order: Order) {
        self.active_orders.insert(order.client_order_id(), order);
    }

    pub fn register_orders(&mut self, orders: &[Order]) {
        self.active_orders
            .extend(orders.iter().copied().map(|o| (o.client_order_id(), o)));
    }

    pub fn get_active_order(&self, id: &Uuid) -> Option<&Order> {
        self.active_orders.get(id)
    }

    pub fn get_active_order_mut(&mut self, id: &Uuid) -> Option<&mut Order> {
        self.active_orders.get_mut(id)
    }

    pub fn complete_order(&mut self, id: Uuid) {
        // TODO: add warnings for duplicate
        if self.active_orders.remove(&id).is_some() {
            self.hist_orders.insert(id);
        }
    }

    pub fn stale_order_ids(&self, max_age: Duration) -> Vec<Uuid> {
        let now = Utc::now();

        self.active_orders
            .iter()
            .filter(|(_, order)| now.signed_duration_since(order.last_update_ts()) >= max_age)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn on_book_ticker_received(&mut self, book_ticker: BookTicker) {
        let bid_level = Level::from((book_ticker.bid_price(), book_ticker.bid_qty()));
        let ask_level = Level::from((book_ticker.ask_price(), book_ticker.ask_qty()));
        self.bbo_level = Some((bid_level, ask_level));
    }

    pub fn on_update_received(
        &mut self,
        update_event: &OrderTradeUpdateEvent,
    ) -> TradingCoreResult<()> {
        use TradingCoreError as Err;
        use data::binance::account::ExecutionType as E;
        let client_id = update_event.client_order_id();

        let order = self.active_orders.get_mut(&client_id).ok_or_else(|| {
            // TODO: more robust
            if self.hist_orders.contains(&client_id) {
                Err::Unknown(format!("Order has been removed {}", client_id))
            } else {
                Err::Unknown(format!("Untracked order {}", client_id))
            }
        })?;

        order.on_update_received(update_event);
        match update_event.exec_type() {
            reason @ (E::Canceled | E::Calculated | E::Expired) => {
                debug!(%client_id, %reason, "Order removed");
                self.complete_order(client_id);
            }
            E::Trade => {
                self.pnl.on_update_received(update_event);
                self.turnover += update_event.last_filled_amount();
                if update_event.order_status() == OrderStatus::Filled {
                    debug!(%client_id, reason="TRADE", "Order removed");
                    self.complete_order(client_id);
                }
            }
            E::Amendment
                if matches!(
                    update_event.order_status(),
                    OrderStatus::Filled | OrderStatus::Canceled
                ) =>
            {
                debug!(%client_id, reason="AMENDMENT", "Order removed");
                self.complete_order(client_id);
            }
            E::New | E::Amendment => {}
        }
        Ok(())
    }
}
