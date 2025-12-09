use data::config::AccountConfidential;
use data::order::{self, *};
use data::request::RequestOpen;
use rust_decimal::dec;
use tokio::task::coop::RestoreOnPending;
use std::error::Error;
use trading_core::exchange::ExecutionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let account = AccountConfidential::from_csv("test", "./test/test_account_info.csv")?;
    let client = ExecutionClient::new(account);
    dbg!(&client);
    let order_request = RequestOpen::new(
        Symbol::BTCUSDT,
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
