use chrono::{Duration, Utc};
use indexmap::IndexMap;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    models::{Order, OrderBook},
    error::{Result as TradingCoreResult, TradingCoreError},
};
use data::{binance::account::OrderTradeUpdateEvent, order::*};
use tracing::debug;

#[allow(dead_code)]
trait Processor<E> {
    type Output;
    fn process(&mut self, event: E) -> Self::Output;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct State {
    // local order book
    order_books: IndexMap<Symbol, OrderBook>,

    // orders that may still receive updates
    active_orders: IndexMap<Uuid, Order>,

    // TODO: add a buffer for handling rejected orders

    // orders filled/cancelled/failed to sent (life ended)
    hist_orders: Vec<Uuid>,

    // current open positions in USDT (Buy, Sell)
    open_position: (Decimal, Decimal),
}

impl State {
    pub fn new() -> Self {
        Self {
            order_books: IndexMap::with_capacity(1),
            active_orders: IndexMap::with_capacity(64),
            hist_orders: Vec::with_capacity(256),
            open_position: (Decimal::new(0, 0), Decimal::new(0, 0)),
        }
    }

    // Order book management
    pub fn get_order_book(&self, symbol: &Symbol) -> Option<&OrderBook> {
        self.order_books.get(symbol)
    }

    pub fn get_order_book_mut(&mut self, symbol: &Symbol) -> Option<&mut OrderBook> {
        self.order_books.get_mut(symbol)
    }

    pub fn set_order_book(&mut self, symbol: Symbol, ob: OrderBook) {
        self.order_books.insert(symbol, ob);
    }

    pub fn remove_order_book(&mut self, symbol: &Symbol) -> Option<OrderBook> {
        self.order_books.swap_remove(symbol)
    }

    pub fn has_order_book(&self, symbol: &Symbol) -> bool {
        self.order_books.contains_key(symbol)
    }

    // Active order tracking
    pub fn track_order(&mut self, order: Order) {
        self.active_orders.insert(order.client_order_id(), order);
    }

    pub fn get_active_order(&self, id: &Uuid) -> Option<&Order> {
        self.active_orders.get(id)
    }

    pub fn get_active_order_mut(&mut self, id: &Uuid) -> Option<&mut Order> {
        self.active_orders.get_mut(id)
    }

    pub fn complete_order(&mut self, id: Uuid) {
        if self.active_orders.shift_remove(&id).is_some() {
            self.hist_orders.push(id);
        }
    }

    pub fn first_active_id(&self) -> Option<Uuid> {
        self.active_orders.first().map(|(k, _)| *k)
    }

    pub fn stale_order_ids(&self, max_age: Duration) -> Vec<Uuid> {
        let now = Utc::now();

        self.active_orders
            .iter()
            .take_while(|(_, order)| now.signed_duration_since(*order.start_ts()) >= max_age)
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn on_update_received(
        &mut self,
        update_event: OrderTradeUpdateEvent,
    ) -> TradingCoreResult<()> {
        use data::binance::account::ExecutionType::*;
        let client_id = update_event.client_order_id();

        let order =
            self.active_orders
                .get_mut(&client_id)
                .ok_or(TradingCoreError::Unrecoverable(format!(
                    "Untracked order {}",
                    client_id
                )))?; // This should never be an error!

        order.on_update_received(&update_event);
        match update_event.exec_type() {
            reason @ (Canceled | Calculated | Expired) => {
                debug!(%client_id, %reason, "Order removed");
                self.complete_order(client_id);
            }
            Trade if update_event.order_status() == OrderStatus::Filled => {
                debug!(%client_id, reason="TRADE", "Order removed");
                self.complete_order(client_id);
            }
            Amendment
                if matches!(
                    update_event.order_status(),
                    OrderStatus::Filled | OrderStatus::Canceled
                ) =>
            {
                debug!(%client_id, reason="AMENDMENT", "Order removed");
                self.complete_order(client_id);
            }
            _ => {}
        }
        // TODO: handling open_position
        Ok(())
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
