use data::config::AccountConfidential;
use data::order::{self, *};
use data::request::RequestOpen;
use rust_decimal::dec;
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
        dec!(69),
        dec!(0.69),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
    );

    let _ = dbg!(&client.sign(order_request));

    let _ = dbg!(&client.open_order(order_request).await);
    Ok(())
}
