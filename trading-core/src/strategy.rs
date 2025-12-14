use crate::engine::State;
use crate::models::Order;
use data::order::*;
use rust_decimal::dec;

pub trait Strategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> Option<(Order, Order)>;
}

pub struct QuoteStrategy;

impl Strategy for QuoteStrategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> Option<(Order, Order)> {
        if let  Some((bid, ask)) = state.get_bbo_level(&symbol) {
            let spread = ask.price - bid.price;
            let mid_price = (ask.price + bid.price ) / dec!(2);
            let ask_opx = mid_price + spread / dec!(2);
            let bid_opx = mid_price - spread / dec!(2);
            Some((
                Order::new(symbol, Side::Buy, OrderKind::Limit, bid_opx, dec!(1), TimeInForce::GoodUntilCancel, None),
                Order::new(symbol, Side::Sell, OrderKind::Limit, ask_opx, dec!(1), TimeInForce::GoodUntilCancel, None),
            ))
        } else {
            None
        }
    }
}