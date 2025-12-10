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

pub const TEST_ENDPOINT_REST: &'static str = "https://demo-fapi.binance.com";
pub const ENDPOINT_REST: &'static str = "https://fapi.binance.com";

#[derive(Debug)]
pub struct ExecutionClient {
    symbol: Symbol,
    api_key: String,
    api_secret: String,
    #[allow(dead_code)]
    is_testnet: bool,
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

    pub fn sign(&self, query_string: &str) -> Result<String, Box<dyn Error>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())?;
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        let signed_request = format!("{}&signature={}", query_string, signature);
        Ok(signed_request)
    }

    async fn signed_post(&self, path: &str, body: String) -> Result<Response, Box<dyn Error>> {
        let url = format!("{}{}", self.endpoint, path);
        let response = self
            .http_client
            .post(url)
            .header("X-MBX-APIKEY", &self.api_key)
            .body(body)
            .send()
            .await?;
        Ok(response)
    }

    pub async fn open_order(
        &self,
        request: RequestOpen,
    ) -> Result<(Response, Uuid), Box<dyn Error>> {
        match (request.time_in_force, request.good_till_date) {
            (TimeInForce::GoodUntilDate, None) => {
                return Err("goodTillDate is required for GTD orders".into());
            }
            (TimeInForce::GoodUntilDate, Some(_)) => {}
            (_, Some(_)) => return Err("goodTillDate should only be set for GTD orders".into()),
            _ => {}
        }

        let client_id = Uuid::new_v4();
        let mut query_string = serde_urlencoded::to_string(&request)?;

        // add timestamp & symbol & clienOrderId
        let ts = Self::get_timestamp();
        query_string.push_str(&format!(
            "&symbol={}&timestamp={}&newClientOrderId={}",
            self.symbol, ts, client_id
        ));

        let signed_request = self.sign(&query_string)?;
        let response = self.signed_post("/fapi/v1/order", signed_request).await?;
        Ok((response, client_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use data::{
        order::{OrderKind, Side, TimeInForce},
        response::OpenOrderSuccess,
    };
    use reqwest::Client;
    use rust_decimal::dec;
    use serde_json;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_open_order() {
        let gtd = Utc::now() + Duration::minutes(20);
        let gtd = (gtd.timestamp() * 1000) as u64;
        let order_request = RequestOpen {
            side: Side::Buy,
            price: dec!(69),
            quantity: dec!(1.0),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilDate,
            good_till_date: Some(gtd),
        };

        let client = ExecutionClient::new(
            "test",
            "../test/test_account_info.csv",
            "BNBUSDT".parse().unwrap(),
            Client::new(),
        )
        .expect("Failed to create client");

        let (response, client_order_id) = client
            .open_order(order_request)
            .await
            .expect("Failed to open order");

        let status = response.status();
        let body = response.text().await.expect("Failed to read response body");
        if !status.is_success() {
            println!("{}", body);
            panic!("order failed: status {}", status);
        }

        let success: OpenOrderSuccess =
            serde_json::from_str(&body).expect("Failed to deserialize order response");

        assert!(success.order_id > 0, "Invalid orderId");
        assert_eq!(
            success.client_order_id, client_order_id,
            "clientOrderId does not match"
        );
    }
}
