use data::{
    order::*,
    request::RequestOpen,
    response::OpenOrderSuccess,
    subscription::{BookDepth, StreamEnvelope, WsCommand},
};
use futures_util::{SinkExt, StreamExt};
use reqwest;
use rust_decimal::dec;
use std::error::Error;
use std::time::Duration;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::protocol::{Message, WebSocketConfig},
};
use trading_core::exchange::ExecutionClient;
use url::Url;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_ENDPOINT_WS: &'static str = "wss://fstream.binancefuture.com/stream";
#[allow(dead_code)]
const ENDPOINT_WS: &'static str = "wss://fstream.binance.com/stream";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let url = Url::parse(TEST_ENDPOINT_WS)?;

    let ws_config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(256 * 1024)
        .max_message_size(Some(512 * 1024))
        .max_frame_size(Some(256 * 1024));
    
    let (ws_stream, _) = connect_async_with_config(url.as_str(), Some(ws_config), true).await?;
    println!("Connected to the socket!");

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let subscribe_msg = WsCommand::new("SUBSCRIBE", vec!["btcusdt@depth5@100ms".to_string()], 1);

    ws_sender
        .send(Message::Text(subscribe_msg.to_string().into()))
        .await?;

    println!("Subscribed to btcusdt@depth5@100ms...");

    // listen to the book depth update
    while let Some(msg) = ws_receiver.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Ok(depth) = serde_json::from_str::<BookDepth>(&text) {
                println!("BookDepth update: {:?}", depth);
                break;
            }

            if let Ok(enveloped) = serde_json::from_str::<StreamEnvelope<BookDepth>>(&text) {
                println!("BookDepth update: {:?}", enveloped.data);
                break;
            }

            println!("WS text (unparsed): {}", text);
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
