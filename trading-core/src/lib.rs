use chrono::{DateTime, Utc};
use data::order::*;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, de::Error as DeError};
use std::collections::BTreeMap;
use std::error::Error;
use std::str::FromStr;

pub mod exchange;

type Price = Decimal;
type Quantity = Decimal;

#[derive(Debug)]
pub struct OrderBook {
    symbol: Symbol,
    local_ts: DateTime<Utc>,
    xchg_ts: DateTime<Utc>,
    bids: BTreeMap<Price, Quantity>,
    asks: BTreeMap<Price, Quantity>,
}

impl OrderBook {
    pub fn new(symbol: Symbol) -> Self {
        OrderBook {
            symbol,
            local_ts: Utc::now(),
            xchg_ts: Utc::now(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub async fn from_snapshot(
        symbol: Symbol,
        depth: u16,
        endpoint: &str,
        client: Client,
    ) -> Result<Self, Box<dyn Error>> {
        let url = format!("{endpoint}/fapi/v1/depth?symbol={symbol}&limit={depth}");
        let response = client.get(url).send().await?;

        response.error_for_status_ref()?;
        let snapshot = response.json::<DepthSnapshot>().await?;
        Ok(OrderBook {
            symbol,
            local_ts: Utc::now(),
            xchg_ts: snapshot.xchg_ts,
            bids: snapshot.bids,
            asks: snapshot.asks,
        })
    }
}

/// Helper struct to construct OrderBook from binance snapshot
#[derive(Deserialize)]
struct DepthSnapshot {
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    xchg_ts: DateTime<Utc>,
    #[serde(deserialize_with = "de_side")]
    bids: BTreeMap<Price, Quantity>,
    #[serde(deserialize_with = "de_side")]
    asks: BTreeMap<Price, Quantity>,
}

/// Deserialize arrays of [price, qty] to BtreeMap.
fn de_side<'de, D>(deserializer: D) -> Result<BTreeMap<Price, Quantity>, D::Error>
where
    D: Deserializer<'de>,
{
    // TODO: Using CursorMut API
    // https://users.rust-lang.org/t/deserialising-my-btreemap-with-serde/110328
    // https://github.com/rust-lang/rust/issues/107540
    // Binance depth returns [["price", "qty"], ...]; let serde parse strings into Decimal.
    let raw: Vec<(Price, Quantity)> = Deserialize::deserialize(deserializer)?;
    let mut side = BTreeMap::new();
    side.extend(raw);
    Ok(side)
}
