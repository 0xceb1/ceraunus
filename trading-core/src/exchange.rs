use chrono::Utc;
use data::{config::AccountConfidential, request::RequestOpen};
use derive_more::Constructor;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{error::Error, result};
use reqwest::{Response, self};

#[allow(dead_code)]
const TEST_ENDPOINT_REST : &'static str = "https://testnet.binancefuture.com";
#[allow(dead_code)]
const TEST_ENDPOINT_WS : &'static str = "wss://fstream.binancefuture.com";
#[allow(dead_code)]
const ENDPOINT_REST : &'static str = "https://demo-fapi.binance.com";
#[allow(dead_code)]
const ENDPOINT_WS : &'static str = "https://demo-fapi.binance.com";



#[derive(Debug, Constructor)]
pub struct ExecutionClient {
    confidential: AccountConfidential,
}

impl ExecutionClient {
    pub fn get_timestamp() -> u64 {
        Utc::now().timestamp_millis() as u64
    }

    pub fn sign(&self, request: RequestOpen) -> Result<String, Box<dyn Error>> {
        let mut query_string = serde_urlencoded::to_string(request)?;

        // add timestamp
        let ts = Self::get_timestamp();
        query_string.push_str(&format!("&timestamp={ts}"));

        // add confidential signature
        let mut mac = Hmac::<Sha256>::new_from_slice(self.confidential.api_secret.as_bytes())?;
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
            .post(format!("{TEST_ENDPOINT_REST}/fapi/v1/order"))
            .header("X-MBX-APIKEY", &self.confidential.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
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
    use data::order::{OrderKind, Side, TimeInForce, Symbol};
    use rust_decimal::dec;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_open_order() {
        let gtd = Utc::now() + Duration::minutes(10);
        let gtd = gtd.timestamp_millis() as u64;
        let order_request = RequestOpen {
            symbol : Symbol::BNBUSDT,
            side: Side::Buy,
            price: dec!(1.0),
            quantity: dec!(0.01),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilDate {
                good_till_date: gtd,
            },
        };

        let confidential = AccountConfidential::from_csv("test", "../test/test_account_info.csv")
            .expect("Cannot read confidentials from csv file.");
        let client = ExecutionClient { confidential };

        let res = client.open_order(order_request).await;
        assert!(res.is_ok())
    }
}
