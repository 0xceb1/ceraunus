#![allow(dead_code)]
use chrono::{DateTime, Utc};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(from = "(Decimal, Decimal)")]
struct Level {
    price: Decimal,
    quantity: Decimal,
}

impl From<(Decimal, Decimal)> for Level {
    fn from((price, amount): (Decimal, Decimal)) -> Self {
        Self {
            price,
            quantity: amount,
        }
    }
}

// Models for the untagged strategy (keep the `e` field present).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookDepth {
    #[serde(rename = "e")]
    _event_type: String,
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggTrade {
    #[serde(rename = "e")]
    _event_type: String,
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "a")]
    agg_trade_id: u64,
    #[serde(rename = "p")]
    price: Decimal,
    #[serde(rename = "q")]
    quantity: Decimal,
    #[serde(rename = "f")]
    first_trade_id: u64,
    #[serde(rename = "l")]
    last_trade_id: u64,
    #[serde(rename = "m")]
    is_buyer_market_maker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Trade {
    #[serde(rename = "e")]
    _event_type: String,
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "p")]
    price: Decimal,
    #[serde(rename = "q")]
    quantity: Decimal,
    #[serde(rename = "m")]
    is_buyer_market_maker: bool,
}

// Slim models for the manual tagged strategy (omit the `e` field).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookDepthNoTag {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggTradeNoTag {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "a")]
    agg_trade_id: u64,
    #[serde(rename = "p")]
    price: Decimal,
    #[serde(rename = "q")]
    quantity: Decimal,
    #[serde(rename = "f")]
    first_trade_id: u64,
    #[serde(rename = "l")]
    last_trade_id: u64,
    #[serde(rename = "m")]
    is_buyer_market_maker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeNoTag {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "p")]
    price: Decimal,
    #[serde(rename = "q")]
    quantity: Decimal,
    #[serde(rename = "m")]
    is_buyer_market_maker: bool,
}

// Strategy 1: untagged helper (full payloads).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EventUntagged {
    Depth(()),
    AggTrade(()),
    Trade(()),
}

// Strategy 2: tagged on `"e"` to avoid the intermediate `Value`.
#[derive(Debug, Deserialize)]
#[serde(tag = "e")]
enum EventManual {
    #[serde(rename = "depthUpdate")]
    Depth(()),
    #[serde(rename = "aggTrade")]
    AggTrade(()),
    #[serde(rename = "trade")]
    Trade(()),
}

const DEPTH_JSON: &str = r#"{
    "e": "depthUpdate",
    "E": 1571889248277,
    "T": 1571889248276,
    "s": "BTCUSDT",
    "U": 390497796,
    "u": 390497878,
    "b": [
        ["7403.89","0.002"],
        ["7403.90","3.906"],
        ["7403.90","3.906"],
        ["7403.90","3.906"],
        ["7403.90","3.906"],
        ["7403.90","3.906"],
        ["7403.90","3.906"],
        ["7403.90","3.906"]
    ],
    "a": [
        ["7405.96","3.340"],
        ["7406.63","4.525"],
        ["7406.63","4.525"],
        ["7406.63","4.525"],
        ["7406.63","4.525"],
        ["7406.63","4.525"],
        ["7406.63","4.525"],
        ["7406.63","4.525"]
    ]
}"#;

const AGG_TRADE_JSON: &str = r#"{
    "e": "aggTrade",
    "E": 1621491230000,
    "T": 1621491230001,
    "s": "ETHUSDT",
    "a": 12345,
    "p": "2500.12",
    "q": "1.234",
    "f": 100,
    "l": 110,
    "m": true
}"#;

const TRADE_JSON: &str = r#"{
    "e": "trade",
    "E": 1621491235000,
    "T": 1621491235001,
    "s": "BNBUSDT",
    "t": 7890,
    "p": "600.01",
    "q": "0.75",
    "m": false
}"#;

fn deserialize_untagged(input: &str) -> EventUntagged {
    serde_json::from_str::<EventUntagged>(input).expect("untagged parse failed")
}

fn deserialize_manual(input: &str) -> EventManual {
    serde_json::from_str::<EventManual>(input).expect("manual parse failed")
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_deserialize");
    for (name, payload) in [
        ("depth", DEPTH_JSON),
        ("agg_trade", AGG_TRADE_JSON),
        ("trade", TRADE_JSON),
    ] {
        group.bench_with_input(BenchmarkId::new("untagged", name), payload, |b, input| {
            b.iter(|| black_box(deserialize_untagged(input)));
        });
        group.bench_with_input(BenchmarkId::new("manual_e", name), payload, |b, input| {
            b.iter(|| black_box(deserialize_manual(input)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_deserialization);
criterion_main!(benches);
