use crate::engine::State;
use crate::models::Order;
use data::order::*;
use rust_decimal::Decimal;
use smallvec::SmallVec;

pub trait Strategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> SmallVec<[Order; 2]>;
}

pub struct QuoteStrategy;

impl Strategy for QuoteStrategy {
    fn generate_quotes(symbol: Symbol, state: &State) -> SmallVec<[Order; 2]> {
        if let Some((bid, ask)) = state.bbo_level {
            let spread = ask.price - bid.price;
            let mid_price = (ask.price + bid.price) / Decimal::TWO;
            let ask_opx = mid_price + spread / Decimal::TWO;
            let bid_opx = mid_price - spread / Decimal::TWO;

            SmallVec::from_slice(&[
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
            ])
        } else {
            SmallVec::new()
        }
    }
}
