use chrono::Utc;
#[allow(unused_imports)]
use data::{
    order::{self, Symbol::SOLUSDT, *},
    request::RequestOpen,
    response::OrderSuccessResp,
    subscription::{Command, Depth, Event, StreamSpec, WsSession},
};
use reqwest;
#[allow(unused_imports)]
use rust_decimal::dec;
use std::time::Duration;
use std::{error::Error, future::Future, pin::Pin};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
#[allow(unused_imports)]
use tracing::{self, debug, error, info, warn};
use tracing_subscriber;
#[allow(unused_imports)]
use trading_core::{
    OrderBook,
    exchange::{Client, TEST_ENDPOINT_REST},
};
use url::Url;

const ACCOUNT_NAME: &'static str = "test";
const ACCOUNT_INFO_PATH: &'static str = "./test/test_account_info.csv";

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const ORDERBOOK_TRIGGER_SECS: i64 = 10;
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

    let client = Client::new(ACCOUNT_NAME, ACCOUNT_INFO_PATH, SOLUSDT, http.clone())
        .ok_or("Failed to build client.")?;

    let listen_key = client.get_listen_key().await?;

    let url = Url::parse(TEST_ENDPOINT_WS)?;
    let user_url = Url::parse(&format!("{}/{}", TEST_ENDPOINT_WS, listen_key))?;

    let ws_config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(256 * 1024)
        .max_message_size(Some(512 * 1024))
        .max_frame_size(Some(256 * 1024));

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (evt_tx, mut evt_rx) = mpsc::channel(1024);
    let (user_cmd_tx, user_cmd_rx) = mpsc::channel(32);
    let (user_evt_tx, mut user_evt_rx) = mpsc::channel(1024);

    let ws = WsSession::new(url, ws_config, cmd_rx, evt_tx);
    let user_ws = WsSession::new(user_url, ws_config.clone(), user_cmd_rx, user_evt_tx);

    ws.spawn();
    user_ws.spawn();

    cmd_tx
        .send(Command::Subscribe(vec![StreamSpec::Depth {
            symbol: SOLUSDT,
            levels: None,
            interval_ms: None,
        }]))
        .await?;

    user_cmd_tx
        .send(Command::Subscribe(vec![StreamSpec::TradeLite]))
        .await?;

    info!("----------INITILIAZATION FINISHED----------");

    let mut depth_buffer: Vec<Depth> = Vec::with_capacity(8);
    let mut order_book: Option<OrderBook> = None;
    let mut snapshot_fut = snapshot_task(SOLUSDT, http.clone(), 1000, Duration::from_millis(1000));
    let mut order_sent = false;
    let loop_start = Utc::now();

    // MAIN EVENT LOOP
    loop {
        tokio::select! {
            // WebSocket events received
            Some(user_event) = user_evt_rx.recv() => match user_event {
                Event::TradeLite(trade_lite) => info!("Trade received, {:?}", trade_lite),
                Event::AggTrade(_) | Event::Trade(_) | Event::Depth(_) => {},
                Event::Raw(bytes) => info!("Raw stream received, {}", bytes),
            },


            Some(event) = evt_rx.recv() => match event {
                Event::Depth(depth) => {
                    debug!(name="Depth received", final_update_id = %depth.final_update_id);
                    if let Some(ob) = order_book.as_mut() {
                        if (depth.last_final_update_id..=depth.final_update_id).contains(&ob.last_update_id) {
                            // TODO: recheck the gap-detection logic here
                            ob.extend(depth);
                        } else {
                            warn!(name="Gap detected in depth updates",
                                %depth.last_final_update_id,
                                %depth.first_update_id,
                                %depth.final_update_id);
                            order_book = None;
                            snapshot_fut = snapshot_task(SOLUSDT, http.clone(), 1000, Duration::from_millis(1000));
                        }
                    } else {
                        // Order book not constructed yet
                        depth_buffer.push(depth);
                        info!(name="Depth pushed to buffer", buffer_size=%&depth_buffer.len());
                    }
                }
                // TODO: we still construct the events even if they are immediately dropped
                Event::AggTrade(_) | Event::Trade(_) | Event::TradeLite(_) => {},
                Event::Raw(bytes) => info!("{}", bytes),
                },

            // SNAPSHOT done
            snapshot_res = &mut snapshot_fut, if order_book.is_none() => {
                let mut ob = snapshot_res?;

                for depth in depth_buffer.drain(..) {
                    if depth.final_update_id < ob.last_update_id {
                        continue; // too old
                    } else {
                        // TODO: we don't check U <= lastUpdateId AND u >= lastUpdateId here
                        ob.extend(depth);
                    }
                }
                info!(name="Order book ready", last_update_id=%ob.last_update_id);
                order_book = Some(ob);
            },

            _ = tokio::time::sleep(Duration::from_secs(1)), if !order_sent => {
                if let Some(ob) = order_book.as_ref() {
                    let elapsed = ob.xchg_ts.signed_duration_since(loop_start);
                    if elapsed.num_seconds() >= ORDERBOOK_TRIGGER_SECS {
                        let order_request = create_order();
                        match client.open_order(order_request).await {
                            Ok(success) => {
                                info!(
                                    name="Open order ACK",
                                    client_order_id=%success.client_order_id,
                                    order_id=%success.order_id
                                );
                                order_sent = true;
                            }
                            Err(err) => {
                                warn!(name="Open order failed", %err);
                            }
                        }
                    }
                }
            },


        }
    }
}

fn snapshot_task(
    symbol: Symbol,
    http: reqwest::Client,
    depth: u16,
    delay: Duration,
) -> Pin<Box<dyn Future<Output = Result<OrderBook, Box<dyn Error>>> + Send>> {
    Box::pin(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        OrderBook::from_snapshot(symbol, depth, TEST_ENDPOINT_REST, http).await
    })
}

fn create_order() -> RequestOpen {
    RequestOpen::new(
        Side::Buy,
        dec!(69),
        dec!(1),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
        None,
    )
}
