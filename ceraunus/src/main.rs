use data::order::*;
use data::request::RequestOpen;
use data::response::OpenOrderSuccess;
use reqwest;
use rust_decimal::dec;
use std::error::Error;
use std::time::Duration;
use trading_core::exchange::ExecutionClient;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
#[allow(dead_code)]
const TEST_ENDPOINT_WS : &'static str = "wss://testnet.binancefuture.com/ws-fapi/v1";
#[allow(dead_code)]
const ENDPOINT_WS : &'static str = "wss://ws-fapi.binance.com/ws-fapi/v1";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // build shared http client
    let http = reqwest::Client::builder()
        .tcp_nodelay(true)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .pool_idle_timeout(IDLE_TIMEOUT)
        .build()?;

    // create a saperate execution client for each symbol
    let client = ExecutionClient::new(
        "test",
        "./test/test_account_info.csv",
        Symbol::BTCUSDT,
        http.clone(),
    )
    .ok_or("Failed to build client.")?;
    dbg!(&client);

    let order_request = RequestOpen::new(
        Side::Buy,
        dec!(69000),
        dec!(0.01),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
    );

    let response = dbg!(client.open_order(order_request).await)?;
    let success: OpenOrderSuccess = response.json().await?;
    println!("Error message: {:?}", success);
    Ok(())
}
