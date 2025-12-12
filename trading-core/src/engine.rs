use std::collections::HashMap;
use rust_decimal::Decimal;
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
    order_books : HashMap<Symbol, OrderBook>,

    // orders that may still receive updates
    active_orders : HashMap<Uuid, Order>, 

    // orders filled/cancelled/failed to sent (life ended)
    hist_orders : Vec<Uuid>,

    // current open positions in USDT (Buy, Sell)  
    open_position : (Decimal, Decimal),
}