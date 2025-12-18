use chrono::{DateTime, Duration, Utc};
use enum_map::EnumMap;
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

#[allow(dead_code)]
trait Processor<E> {
    type Output;
    fn process(&mut self, event: E) -> Self::Output;
}

type BboPair = (Level, Level);

#[allow(dead_code)]
#[derive(Debug)]
pub struct State {
    // best-available ask & bid
    pub bbo_levels: EnumMap<Symbol, Option<BboPair>>, // (bid_level, ask_level)

    // local order book
    pub order_books: EnumMap<Symbol, Option<OrderBook>>,

    // orders that may still receive updates
    active_orders: FxHashMap<Uuid, Order>,

    // TODO: add a buffer for handling rejected orders

    // orders filled/cancelled/failed to sent (life ended)
    hist_orders: FxHashSet<Uuid>,

    // current net position in quantity
    position: EnumMap<Symbol, Decimal>,

    start_time: DateTime<Utc>,

    // total traded amount in USDT
    turnover: Decimal,
}

impl State {
    pub fn new() -> Self {
        Self {
            bbo_levels: EnumMap::from_fn(|_| None),
            order_books: EnumMap::from_fn(|_| None),
            active_orders: FxHashMap::with_capacity_and_hasher(128, FxBuildHasher),
            hist_orders: FxHashSet::with_capacity_and_hasher(1024, FxBuildHasher),
            position: EnumMap::default(),
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

    pub fn get_position(&self, symbol: Symbol) -> Decimal {
        self.position[symbol]
    }

    // Order book management
    pub fn remove_order_book(&mut self, symbol: Symbol) {
        self.order_books[symbol] = None;
    }

    pub fn has_order_book(&self, symbol: Symbol) -> bool {
        self.order_books[symbol].is_some()
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
        self.bbo_levels[book_ticker.symbol()] = Some((bid_level, ask_level));
    }

    pub fn on_update_received(
        &mut self,
        update_event: OrderTradeUpdateEvent,
    ) -> TradingCoreResult<()> {
        use data::binance::account::ExecutionType as E;
        let client_id = update_event.client_order_id();

        let order = self.active_orders.get_mut(&client_id).ok_or_else(|| {
            // TODO: more robust
            if self.hist_orders.contains(&client_id) {
                TradingCoreError::Unknown(format!("Order has been removed {}", client_id))
            } else {
                TradingCoreError::Unknown(format!("Untracked order {}", client_id))
            }
        })?;

        order.on_update_received(&update_event);
        match update_event.exec_type() {
            reason @ (E::Canceled | E::Calculated | E::Expired) => {
                debug!(%client_id, %reason, "Order removed");
                self.complete_order(client_id);
            }
            E::Trade => {
                let symbol = update_event.symbol();
                let qty = update_event.last_filled_qty();
                match update_event.side() {
                    Side::Buy => self.position[symbol] += qty,
                    Side::Sell => self.position[symbol] -= qty,
                }
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

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
