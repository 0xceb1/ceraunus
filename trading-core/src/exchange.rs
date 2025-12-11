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

    async fn signed_delete(&self, path: &str, body: String) -> Result<Response, Box<dyn Error>> {
        // For Binance signed DELETE endpoints, send the signed query on the URL.
        let url = format!("{}{}?{}", self.endpoint, path, body);
        let response = self
            .http_client
            .delete(url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;
        Ok(response)
    }

    pub async fn open_order(
        &self,
        request: RequestOpen,
    ) -> Result<(Response, Uuid), Box<dyn Error>> {
        match (request.time_in_force, request.good_till_date) {
            (TimeInForce::GoodUntilDate, Some(_)) => {}
            (TimeInForce::GoodUntilDate, None) | (_, Some(_)) => {
                return Err("Unmatched timeInForce and goodTilDate".into());
            }
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

    pub async fn cancel_order(&self, client_id: Uuid) -> Result<Response, Box<dyn Error>> {
        let query_string = format!(
            "symbol={}&origClientOrderId={}&timestamp={}",
            self.symbol,
            client_id,
            Self::get_timestamp()
        );
        let signed_request = self.sign(&query_string)?;
        let response = self.signed_delete("/fapi/v1/order", signed_request).await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use data::{
        order::{OrderKind, Side, TimeInForce},
        response::OrderSuccessResp,
    };
    use reqwest::Client;
    use rust_decimal::dec;
    use serde_json;

    fn make_client() -> ExecutionClient {
        ExecutionClient::new(
            "test",
            "../test/test_account_info.csv",
            "BNBUSDT".parse().unwrap(),
            Client::new(),
        )
        .expect("Failed to create client")
    }

    fn make_open_request() -> RequestOpen {
        let gtd = Utc::now() + Duration::minutes(20);
        let gtd = (gtd.timestamp() * 1000) as u64;
        RequestOpen {
            side: Side::Buy,
            price: dec!(69),
            quantity: dec!(1.0),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilDate,
            good_till_date: Some(gtd),
        }
    }

    #[tokio::test()]
    async fn test_open_order() {
        let order_request = make_open_request();
        let client = make_client();

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

        let success: OrderSuccessResp =
            serde_json::from_str(&body).expect("Failed to deserialize order response");

        assert!(success.order_id > 0, "Invalid orderId");
        assert_eq!(
            success.client_order_id, client_order_id,
            "clientOrderId does not match"
        );
    }

    #[tokio::test()]
    async fn test_cancel_order() {
        let order_request = make_open_request();
        let client = make_client();

        let (open_response, client_order_id) = client
            .open_order(order_request)
            .await
            .expect("Failed to open order");

        let open_status = open_response.status();
        let open_body = open_response
            .text()
            .await
            .expect("Failed to read open response body");
        if !open_status.is_success() {
            println!("{}", open_body);
            panic!("order failed: status {}", open_status);
        }

        let cancel_response = client
            .cancel_order(client_order_id)
            .await
            .expect("Failed to cancel order");

        let cancel_status = cancel_response.status();
        let cancel_body = cancel_response
            .text()
            .await
            .expect("Failed to read cancel response body");
        if !cancel_status.is_success() {
            println!("{}", cancel_body);
            panic!("cancel failed: status {}", cancel_status);
        }

        let canceled: serde_json::Value =
            serde_json::from_str(&cancel_body).expect("Failed to deserialize cancel response");

        assert_eq!(
            canceled.get("clientOrderId").and_then(|v| v.as_str()),
            Some(client_order_id.to_string().as_str()),
            "clientOrderId does not match after cancel"
        );
    }
}
