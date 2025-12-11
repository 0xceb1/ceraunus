#[allow(unused_imports)]
use data::{
    order::{self, *},
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
use tracing::{self, error, info, warn};
use tracing_subscriber;
#[allow(unused_imports)]
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

    ws.spawn();

    cmd_tx
        .send(Command::Subscribe(vec![StreamSpec::Depth {
            symbol: "BTCUSDT".parse()?,
            levels: None,
            interval_ms: None,
        }]))
        .await?;

    info!("Subscribed to depth streams");

    let mut depth_buffer: Vec<Depth> = Vec::with_capacity(8);
    let mut order_book: Option<OrderBook> = None;
    let mut snapshot_fut = snapshot_task(http.clone(), 1000, Duration::from_millis(1000));

    loop {
        tokio::select! {
            // WebSocket events received
            Some(event) = evt_rx.recv() => match event {
                Event::Depth(depth) => {
                    info!(name="Depth received", final_update_id = %depth.final_update_id);
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
                            snapshot_fut = snapshot_task(http.clone(), 1000, Duration::from_millis(1000));
                        }
                    } else {
                        // Order book not constructed yet
                        depth_buffer.push(depth);
                        info!(name="Depth pushed to buffer", buffer_size=%&depth_buffer.len());
                    }
                }
                Event::AggTrade(_) | Event::Trade(_) => {},
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
            }
        }
    }

    //     if cnt > 50 {
    //         let _ = cmd_tx.send(Command::Shutdown).await;
    //         break;
    //     }
    // }

    // create a saperate execution client for each symbol
    // let client = ExecutionClient::new(
    //     "test",
    //     "./test/test_account_info.csv",
    //     "SOLUSDT".parse()?,
    //     http.clone(),
    // )
    // .ok_or("Failed to build client.")?;

    // info!("Execution client built: {:?}", &client);

    // let order_request = RequestOpen::new(
    //     Side::Buy,
    //     dec!(69),
    //     dec!(1),
    //     OrderKind::Limit,
    //     TimeInForce::GoodUntilCancel,
    //     None,
    // );

    // let (response, _client_order_id) = client.open_order(order_request).await?;

    // let success: OrderSuccessResp = response.json().await?;
    // info!("Order placement ACK: {:?}", success);
    // Ok(())
}

fn snapshot_task(
    http: reqwest::Client,
    depth: u16,
    delay: Duration,
) -> Pin<Box<dyn Future<Output = Result<OrderBook, Box<dyn Error>>> + Send>> {
    Box::pin(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        OrderBook::from_snapshot(Symbol::BTCUSDT, depth, TEST_ENDPOINT_REST, http).await
    })
}
