use data::{
    order::{self, *},
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
#[allow(unused_imports)]
use tracing::{self, error, info, warn};
use tracing_subscriber;
use trading_core::{
    OrderBook,
    exchange::{ExecutionClient, TEST_ENDPOINT_REST},
};
use url::Url;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_ENDPOINT_WS: &'static str = "wss://fstream.binancefuture.com/ws";
#[allow(dead_code)]
const ENDPOINT_WS: &'static str = "wss://fstream.binance.com/ws";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure tracing subscriber
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(false)
        // .pretty()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // build shared http client
    let http = reqwest::Client::builder()
        .tcp_nodelay(true)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .pool_idle_timeout(IDLE_TIMEOUT)
        .build()?;

    let url = Url::parse(TEST_ENDPOINT_WS)?;

    let ws_config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(256 * 1024)
        .max_message_size(Some(512 * 1024))
        .max_frame_size(Some(256 * 1024));

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (evt_tx, mut evt_rx) = mpsc::channel(1024);

    let ws = WsSession::new(url, ws_config, cmd_rx, evt_tx);

    let order_book: OrderBook = OrderBook::from_snapshot(
        Symbol::BTCUSDT,
        100,
        TEST_ENDPOINT_REST,
        http.clone(),
    )
    .await?;

    dbg!(order_book);

    ws.spawn();

    cmd_tx
        .send(Command::Subscribe(vec![
            StreamSpec::Depth {
                symbol: "BTCUSDT".parse()?,
                levels: 20,
                interval_ms: 100,
            },
            StreamSpec::Trade {
                symbol: "BNBUSDT".parse()?,
            },
            StreamSpec::AggTrade {
                symbol: "BTCUSDT".parse()?,
            },
        ]))
        .await?;

    info!("Subscribed to depth streams");

    let mut cnt = 0;
    while let Some(event) = evt_rx.recv().await {
        match event {
            Event::Depth(depth) => {
                info!("BookDepth update received: {:?}", depth);
                cnt += 1;
            }
            Event::Trade(trade) => {
                info!("Trade update received: {:?}", trade);
                cnt += 1;
            }
            Event::AggTrade(agg_trade) => {
                info!("AggTrade update received: {:?}", agg_trade);
                cnt += 1;
            }
            Event::Raw(text) => {
                error!("Unknown WS text: {}", text);
            }
        }
        if cnt > 10 {
            let _ = cmd_tx.send(Command::Shutdown).await;
            break;
        }
    }

    // create a saperate execution client for each symbol
    let client = ExecutionClient::new(
        "test",
        "./test/test_account_info.csv",
        "SOLUSDT".parse()?,
        http.clone(),
    )
    .ok_or("Failed to build client.")?;

    info!("Execution client built: {:?}", &client);

    let order_request = RequestOpen::new(
        Side::Buy,
        dec!(69),
        dec!(1),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
        None,
    );

    let (response, _client_order_id) = client.open_order(order_request).await?;

    let success: OpenOrderSuccess = response.json().await?;
    info!("Order placement ACK: {:?}", success);
    Ok(())
}
