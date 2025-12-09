use data::order::*;
use data::request::RequestOpen;
use rust_decimal::dec;
use std::error::Error;
use std::time::Duration;
use reqwest;
use trading_core::exchange::ExecutionClient;

const IDLE_TIMEOUT : Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT : Duration = Duration::from_secs(3);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // build shared http client
    let http = reqwest::Client::builder()
    .tcp_nodelay(true)
    .timeout(HTTP_REQUEST_TIMEOUT)
    .pool_idle_timeout(IDLE_TIMEOUT)
    .build()?;

    // create a saperate execution client for each symbol
    let client = ExecutionClient::new("test", "./test/test_account_info.csv", Symbol::BTCUSDT, http.clone())
    .ok_or("Failed to build client.")?;
    dbg!(&client);

    let order_request = RequestOpen::new(
        Side::Buy,
        dec!(69000),
        dec!(0.01),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
    );

    let _ = dbg!(&client.sign(order_request));

    let response = dbg!(client.open_order(order_request).await);
    let response = response?;
    println!("Error message: {}", response.text().await?);
    Ok(())
}
