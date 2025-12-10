use data::{
    order::*,
    request::RequestOpen,
    response::OpenOrderSuccess,
    subscription::{Command, Event, StreamSpec, WsSession},
};
use reqwest;
use rust_decimal::dec;
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use trading_core::exchange::ExecutionClient;
use url::Url;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_ENDPOINT_WS: &'static str = "wss://fstream.binancefuture.com/ws";
#[allow(dead_code)]
const ENDPOINT_WS: &'static str = "wss://fstream.binance.com/ws";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = Url::parse(TEST_ENDPOINT_WS)?;

    let ws_config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(256 * 1024)
        .max_message_size(Some(512 * 1024))
        .max_frame_size(Some(256 * 1024));

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (evt_tx, mut evt_rx) = mpsc::channel(1024);

    WsSession::new(url, ws_config, cmd_rx, evt_tx).spawn();

    cmd_tx
        .send(Command::Subscribe(vec![StreamSpec::Depth {
            symbol: "BTCUSDT".parse()?,
            levels: 5,
            interval_ms: 100,
        }]))
        .await?;

    println!("Subscribed to depth streams...");

    let mut cnt = 0;
    while let Some(event) = evt_rx.recv().await {
        match event {
            Event::Depth(depth) => {
                println!("BookDepth update: {:?}", depth);
                cnt += 1;
            }
            Event::Raw(text) => {
                println!("WS text (unparsed): {}", text);
            }
        }
        if cnt > 5 {
            let _ = cmd_tx.send(Command::Shutdown).await;
            break;
        }
    }

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
        "SOLUSDT".parse()?,
        http.clone(),
    )
    .ok_or("Failed to build client.")?;
    dbg!(&client);

    let order_request = RequestOpen::new(
        Side::Buy,
        dec!(69),
        dec!(1),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
        None,
    );

    let (response, _client_order_id) = dbg!(client.open_order(order_request).await)?;
    let success: OpenOrderSuccess = response.json().await?;
    println!("Error message: {:?}", success);
    Ok(())
}
