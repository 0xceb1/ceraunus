#![allow(dead_code)]
use chrono::{DateTime, Utc};
use criterion::{Criterion, criterion_group, criterion_main};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use std::hint::black_box;

const ORDER_TRADE_UPDATE_JSON: &str = r#"{
    "e": "ORDER_TRADE_UPDATE",
    "E": 1568879465651,
    "T": 1568879465650,
    "o": {
        "s": "BTCUSDT",
        "c": "TEST",
        "S": "SELL",
        "o": "TRAILING_STOP_MARKET",
        "f": "GTC",
        "q": "0.001",
        "p": "0",
        "ap": "0",
        "sp": "7103.04",
        "x": "NEW",
        "X": "NEW",
        "i": 8886774,
        "l": "0",
        "z": "0",
        "L": "0",
        "N": "USDT",
        "n": "0",
        "T": 1568879465650,
        "t": 0,
        "b": "0",
        "a": "9.91",
        "m": false,
        "R": false,
        "wt": "CONTRACT_PRICE",
        "ot": "TRAILING_STOP_MARKET",
        "ps": "LONG",
        "cp": false,
        "AP": "7476.89",
        "cr": "5.0",
        "pP": false,
        "si": 0,
        "ss": 0,
        "rp": "0",
        "V": "EXPIRE_TAKER",
        "pm": "OPPONENT",
        "gtd": 0,
        "er": "0"
    }
}"#;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OrderKind {
    Limit,
    Market,
    Stop,
    StopMarket,
    TakeProfit,
    TakeProfitMarket,
    TrailingStopMarket,
    Liquidation,
}

#[derive(Debug, Deserialize, Clone, Copy)]
enum TimeInForce {
    #[serde(rename = "GTC")]
    GoodUntilCancel,
    #[serde(rename = "GTD")]
    GoodUntilDate,
    #[serde(rename = "GTX")]
    GoodTillCrossing,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
enum ExecutionType {
    New,
    Canceled,
    Calculated,
    Expired,
    Trade,
    Amendment,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    ExpiredInMatch,
}

// ============================================================================
// Approach 1: Wrapper struct (original)
// ============================================================================

#[derive(Debug, Deserialize)]
struct OrderTradeUpdateWrapper {
    #[serde(rename = "o")]
    order: OrderTradeUpdateInnerWrapper,
}

#[derive(Debug, Deserialize)]
struct OrderTradeUpdateInnerWrapper {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    client_order_id: String,
    #[serde(rename = "S")]
    side: Side,
    #[serde(rename = "o")]
    order_type: OrderKind,
    #[serde(rename = "f")]
    time_in_force: TimeInForce,
    #[serde(rename = "q")]
    orig_qty: Decimal,
    #[serde(rename = "p")]
    orig_price: Decimal,
    #[serde(rename = "ap")]
    avg_price: Decimal,
    #[serde(rename = "x")]
    execution_type: ExecutionType,
    #[serde(rename = "X")]
    order_status: OrderStatus,
    #[serde(rename = "i")]
    order_id: u64,
    #[serde(rename = "l")]
    last_filled_qty: Decimal,
    #[serde(rename = "z")]
    filled_qty: Decimal,
    #[serde(rename = "L")]
    last_filled_price: Decimal,
    #[serde(rename = "n")]
    commission: Decimal,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    order_trade_time: DateTime<Utc>,
    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "m")]
    is_maker: bool,
    #[serde(rename = "rp")]
    realized_profit: Decimal,
}

// ============================================================================
// Approach 2: Custom deserializer (current implementation)
// ============================================================================

#[derive(Debug)]
struct OrderTradeUpdateCustom {
    symbol: String,
    client_order_id: String,
    side: Side,
    order_type: OrderKind,
    time_in_force: TimeInForce,
    orig_qty: Decimal,
    orig_price: Decimal,
    avg_price: Decimal,
    execution_type: ExecutionType,
    order_status: OrderStatus,
    order_id: u64,
    last_filled_qty: Decimal,
    filled_qty: Decimal,
    last_filled_price: Decimal,
    commission: Decimal,
    order_trade_time: DateTime<Utc>,
    trade_id: u64,
    is_maker: bool,
    realized_profit: Decimal,
}

#[derive(Deserialize)]
struct OrderTradeUpdateRaw {
    #[serde(rename = "o")]
    order: OrderTradeUpdateInnerRaw,
}

#[derive(Deserialize)]
struct OrderTradeUpdateInnerRaw {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    client_order_id: String,
    #[serde(rename = "S")]
    side: Side,
    #[serde(rename = "o")]
    order_type: OrderKind,
    #[serde(rename = "f")]
    time_in_force: TimeInForce,
    #[serde(rename = "q")]
    orig_qty: Decimal,
    #[serde(rename = "p")]
    orig_price: Decimal,
    #[serde(rename = "ap")]
    avg_price: Decimal,
    #[serde(rename = "x")]
    execution_type: ExecutionType,
    #[serde(rename = "X")]
    order_status: OrderStatus,
    #[serde(rename = "i")]
    order_id: u64,
    #[serde(rename = "l")]
    last_filled_qty: Decimal,
    #[serde(rename = "z")]
    filled_qty: Decimal,
    #[serde(rename = "L")]
    last_filled_price: Decimal,
    #[serde(rename = "n")]
    commission: Decimal,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    order_trade_time: DateTime<Utc>,
    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "m")]
    is_maker: bool,
    #[serde(rename = "rp")]
    realized_profit: Decimal,
}

impl<'de> Deserialize<'de> for OrderTradeUpdateCustom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = OrderTradeUpdateRaw::deserialize(deserializer)?;
        let o = raw.order;
        Ok(OrderTradeUpdateCustom {
            symbol: o.symbol,
            client_order_id: o.client_order_id,
            side: o.side,
            order_type: o.order_type,
            time_in_force: o.time_in_force,
            orig_qty: o.orig_qty,
            orig_price: o.orig_price,
            avg_price: o.avg_price,
            execution_type: o.execution_type,
            order_status: o.order_status,
            order_id: o.order_id,
            last_filled_qty: o.last_filled_qty,
            filled_qty: o.filled_qty,
            last_filled_price: o.last_filled_price,
            commission: o.commission,
            order_trade_time: o.order_trade_time,
            trade_id: o.trade_id,
            is_maker: o.is_maker,
            realized_profit: o.realized_profit,
        })
    }
}

fn deserialize_wrapper(input: &str) -> OrderTradeUpdateWrapper {
    serde_json::from_str(input).expect("wrapper parse failed")
}

fn deserialize_custom(input: &str) -> OrderTradeUpdateCustom {
    serde_json::from_str(input).expect("custom parse failed")
}

fn bench_order_trade_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_trade_update_deserialize");

    group.bench_function("wrapper", |b| {
        b.iter(|| black_box(deserialize_wrapper(ORDER_TRADE_UPDATE_JSON)));
    });

    group.bench_function("custom_deser", |b| {
        b.iter(|| black_box(deserialize_custom(ORDER_TRADE_UPDATE_JSON)));
    });

    group.finish();
}

criterion_group!(benches, bench_order_trade_update);
criterion_main!(benches);
