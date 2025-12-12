use crate::error::{ApiError, ClientError, MessageCodecError, Result};
use chrono::Utc;
use data::{
    DataError,
    binance::request::RequestOpen,
    binance::response::OrderSuccessResp,
    config::AccountConfidential,
    order::{Symbol, TimeInForce},
};
use hmac::{Hmac, Mac};
use reqwest::{self, Response, StatusCode};
use serde_json::Value;
use sha2::Sha256;
use std::path::Path;
use uuid::Uuid;

pub const TEST_ENDPOINT_REST: &'static str = "https://demo-fapi.binance.com";
pub const ENDPOINT_REST: &'static str = "https://fapi.binance.com";

#[derive(Debug)]
pub struct Client {
    symbol: Symbol,
    pub api_key: String,
    api_secret: String,
    #[allow(dead_code)]
    is_testnet: bool,
    http_client: reqwest::Client,
    endpoint: String,
}

fn map_api_error(status: StatusCode, body: String) -> ApiError {
    // TODO: parsing status & body correctly
    match status {
        StatusCode::TOO_MANY_REQUESTS => ApiError::RateLimit { status, body },
        _ => ApiError::Unknown { status, body },
    }
}

impl Client {
    pub fn new(
        name: &str,
        csv_path: impl AsRef<Path>,
        symbol: Symbol,
        http_client: reqwest::Client,
    ) -> Result<Self> {
        let confidential = AccountConfidential::from_csv(name, csv_path)?;
        let client = match confidential.is_testnet() {
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
        Ok(client)
    }

    fn now_u64() -> u64 {
        Utc::now().timestamp_millis() as u64
    }

    pub fn sign(&self, query_string: &str) -> Result<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .map_err(MessageCodecError::from)?;
        mac.update(query_string.as_bytes());
        let result = mac.finalize();
        let signature = hex::encode(result.into_bytes());

        let signed_request = format!("{}&signature={}", query_string, signature);
        Ok(signed_request)
    }

    async fn signed_post(&self, path: &str, body: String) -> Result<Response> {
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

    async fn signed_put(&self, path: &str, body: String) -> Result<Response> {
        let url = format!("{}{}", self.endpoint, path);
        let response = self
            .http_client
            .put(url)
            .header("X-MBX-APIKEY", &self.api_key)
            .body(body)
            .send()
            .await?;
        Ok(response)
    }

    async fn signed_delete(&self, path: &str, body: String) -> Result<Response> {
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

    pub async fn get_listen_key(&self) -> Result<String> {
        let signed_request = self.sign("")?;
        let response = self
            .signed_post("/fapi/v1/listenKey", signed_request)
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            let api_err = map_api_error(status, body);
            return Err(ClientError::from(api_err));
        }

        let listen_key = serde_json::from_str::<Value>(&body)?
            .get("listenKey")
            .and_then(|v| v.as_str())
            .ok_or(MessageCodecError::MissingField("listenKey"))?
            .to_string();

        Ok(listen_key)
    }

    pub async fn keepalive_listen_key(&self) -> Result<String> {
        let signed_request = self.sign("")?;
        let response = self
            .signed_put("/fapi/v1/listenKey", signed_request)
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            let api_err = map_api_error(status, body);
            return Err(ClientError::from(api_err));
        }

        let listen_key = serde_json::from_str::<Value>(&body)?
            .get("listenKey")
            .and_then(|v| v.as_str())
            .ok_or(MessageCodecError::MissingField("listenKey"))?
            .to_string();

        Ok(listen_key)
    }

    pub async fn open_order(&self, request: RequestOpen) -> Result<OrderSuccessResp> {
        match (request.time_in_force(), request.good_till_date()) {
            (TimeInForce::GoodUntilDate, Some(_)) => {}
            (TimeInForce::GoodUntilDate, None) | (_, Some(_)) => {
                return Err(DataError::BadDefinition {
                    reason: "Unmatched timeInForce and goodTilDate",
                }
                .into());
            }
            _ => {}
        }

        // TODO: use copy? maybe benchmark first
        let mut query_string =
            serde_urlencoded::to_string(&request).map_err(MessageCodecError::from)?;

        // add timestamp & symbol & clienOrderId
        let ts = Self::now_u64();
        query_string.push_str(&format!("&symbol={}&timestamp={}", self.symbol, ts));

        let signed_request = self.sign(&query_string)?;
        let response = self.signed_post("/fapi/v1/order", signed_request).await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let api_err = map_api_error(status, body);
            return Err(ClientError::from(api_err));
        }

        let success: OrderSuccessResp = serde_json::from_str(&body)?;
        Ok(success)
    }

    pub async fn cancel_order(&self, client_id: Uuid) -> Result<OrderSuccessResp> {
        let query_string = format!(
            "symbol={}&origClientOrderId={}&timestamp={}",
            self.symbol,
            client_id,
            Self::now_u64()
        );
        let signed_request = self.sign(&query_string)?;
        let response = self.signed_delete("/fapi/v1/order", signed_request).await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            let api_err = map_api_error(status, body);
            return Err(ClientError::from(api_err));
        }

        let success: OrderSuccessResp = serde_json::from_str(&body)?;
        Ok(success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use data::{
        binance::response::OrderSuccessResp,
        order::{OrderKind, OrderStatus, Side, TimeInForce},
    };
    use rust_decimal::dec;

    fn make_client() -> Client {
        Client::new(
            "test",
            "../test/test_account_info.csv",
            "BNBUSDT".parse().unwrap(),
            reqwest::Client::new(),
        )
        .expect("Failed to create client")
    }

    fn make_open_request() -> RequestOpen {
        let gtd = Utc::now() + Duration::minutes(20);
        let gtd = (gtd.timestamp() * 1000) as u64;
        RequestOpen::new(
            Side::Buy,
            dec!(69),
            dec!(1.0),
            OrderKind::Limit,
            Uuid::new_v4(),
            TimeInForce::GoodUntilDate,
            Some(gtd),
        )
    }

    #[tokio::test]
    async fn test_get_listen_key() {
        let client = make_client();
        let listen_key = client
            .get_listen_key()
            .await
            .expect("Failed to fetch listen key");

        println!("listen key: {}", listen_key);
        assert!(!listen_key.is_empty(), "listen key should not be empty");
    }

    #[tokio::test()]
    async fn test_open_order() {
        let order_request = make_open_request();
        let client = make_client();

        let success: OrderSuccessResp = client
            .open_order(order_request)
            .await
            .expect("Failed to open order");

        assert!(success.order_id() > 0, "Invalid orderId");
    }

    #[tokio::test()]
    async fn test_cancel_order() {
        let order_request = make_open_request();
        let client = make_client();
        let client_order_id = order_request.client_order_id();

        let _success: OrderSuccessResp = client
            .open_order(order_request)
            .await
            .expect("Failed to open order");

        let cancel_success = client
            .cancel_order(client_order_id)
            .await
            .expect("Failed to cancel order");

        assert_eq!(cancel_success.status(), OrderStatus::Canceled);
    }
}
