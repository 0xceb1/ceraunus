use chrono::Utc;
use data::{
    config::AccountConfidential,
    order::{Symbol, TimeInForce},
    request::RequestOpen,
};
use hmac::{Hmac, Mac};
use reqwest::{self, Response};
use sha2::Sha256;
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

const TEST_ENDPOINT_REST: &'static str = "https://demo-fapi.binance.com";
const ENDPOINT_REST: &'static str = "https://fapi.binance.com";

#[derive(Debug)]
pub struct ExecutionClient {
    symbol: Symbol,
    api_key: String,
    api_secret: String,
    #[allow(dead_code)]
    is_testnet: bool,
    #[allow(dead_code)]
    http_client: reqwest::Client,
    endpoint: String,
}

impl ExecutionClient {
    pub fn new(
        name: &str,
        csv_path: impl AsRef<Path>,
        symbol: Symbol,
        http_client: reqwest::Client,
    ) -> Option<Self> {
        let confidential = AccountConfidential::from_csv(name, csv_path).ok()?;
        let client = match confidential.is_testnet {
            true => Self {
                symbol,
                api_key: confidential.api_key,
                api_secret: confidential.api_secret,
                is_testnet: true,
                http_client,
                endpoint: String::from(TEST_ENDPOINT_REST),
            },
            false => Self {
                symbol,
                api_key: confidential.api_key,
                api_secret: confidential.api_secret,
                is_testnet: false,
                http_client,
                endpoint: String::from(ENDPOINT_REST),
            },
        };
        Some(client)
    }

    fn get_timestamp() -> u64 {
        Utc::now().timestamp_millis() as u64
    }

    pub fn sign(&self, request: RequestOpen) -> Result<String, Box<dyn Error>> {
        match (request.time_in_force, request.good_till_date) {
            (TimeInForce::GoodUntilDate, None) => {
                return Err("goodTillDate is required for GTD orders".into());
            }
            (TimeInForce::GoodUntilDate, Some(_)) => {}
            (_, Some(_)) => return Err("goodTillDate should only be set for GTD orders".into()),
            _ => {}
        }
        let mut query_string = serde_urlencoded::to_string(request)?;

        // add timestamp & symbol & clienOrderId
        let ts = Self::get_timestamp();
        let client_id = Uuid::new_v4().to_string();
        println!("client_id: {}", client_id);
        query_string.push_str(&format!(
            "&symbol={}&timestamp={}&newClientOrderId={}",
            self.symbol, ts, client_id
        ));

        // add confidential signature
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())?;
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        let signed_request = format!("{}&signature={}", query_string, signature);
        Ok(signed_request)
    }

    pub async fn open_order(&self, request: RequestOpen) -> Result<Response, Box<dyn Error>> {
        let signed_request = self.sign(request)?;
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/fapi/v1/order", self.endpoint))
            .header("X-MBX-APIKEY", &self.api_key)
            .body(signed_request)
            .send()
            .await;
        response.map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use data::order::{OrderKind, Side, Symbol, TimeInForce};
    use reqwest::Client;
    use rust_decimal::dec;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_open_order() {
        let gtd = Utc::now() + Duration::minutes(10);
        let gtd = gtd.timestamp_millis() as u64;
        let order_request = RequestOpen {
            side: Side::Buy,
            price: dec!(1.0),
            quantity: dec!(0.01),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilDate,
            good_till_date: Some(gtd),
        };

        let client = ExecutionClient::new(
            "test",
            "../test/test_account_info.csv",
            Symbol::BTCUSDT,
            Client::new(),
        )
        .expect("Failed to create execution client");

        let res = client.open_order(order_request).await;
        assert!(res.is_ok())
    }
}
