use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use crate::{OrderBook, account::Order};
use data::order::Symbol;

#[allow(dead_code)]
trait Processor<E> {
    type Output;
    fn process(&mut self, event: E) -> Self::Output;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct State {
    // local order book
    order_books: HashMap<Symbol, OrderBook>,

    // orders that may still receive updates
    active_orders: HashMap<Uuid, Order>,

    // orders filled/cancelled/failed to sent (life ended)
    hist_orders: Vec<Uuid>,

    // current open positions in USDT (Buy, Sell)
    open_position: (Decimal, Decimal),
}

impl State {
    pub fn new() -> Self {
        Self {
            order_books: HashMap::with_capacity(1),
            active_orders: HashMap::with_capacity(64),
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
        self.order_books.remove(symbol)
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

    pub fn complete_order(&mut self, id: Uuid) {
        if self.active_orders.remove(&id).is_some() {
            self.hist_orders.push(id);
        }
    }

    pub fn first_active_id(&self) -> Option<Uuid> {
        self.active_orders.keys().next().copied()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
