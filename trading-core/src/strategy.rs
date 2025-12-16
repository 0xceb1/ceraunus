use crate::engine::State;
use crate::models::Order;
use data::order::*;
use rust_decimal::Decimal;

pub trait Strategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> Option<(Order, Order)>;
}

pub struct QuoteStrategy;

impl Strategy for QuoteStrategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> Option<(Order, Order)> {
        if let Some((bid, ask)) = state.bbo_levels[symbol] {
            let spread = ask.price - bid.price;
            let mid_price = (ask.price + bid.price) / Decimal::TWO;
            let ask_opx = mid_price + spread / Decimal::TWO;
            let bid_opx = mid_price - spread / Decimal::TWO;
            Some((
                Order::new(
                    symbol,
                    Side::Buy,
                    OrderKind::Limit,
                    bid_opx,
                    Decimal::ONE,
                    TimeInForce::GoodUntilCancel,
                    None,
                ),
                Order::new(
                    symbol,
                    Side::Sell,
                    OrderKind::Limit,
                    ask_opx,
                    Decimal::ONE,
                    TimeInForce::GoodUntilCancel,
                    None,
                ),
            ))
        } else {
            None
        }
    }
}
