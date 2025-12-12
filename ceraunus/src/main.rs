use anyhow::Result;
use chrono::Utc;
use data::{
    binance::market::Depth,
    binance::request::RequestOpen,
    binance::subscription::{AccountStream, MarketStream, StreamCommand, StreamSpec, WsSession},
    order::{Symbol::SOLUSDT, *},
};
use reqwest;
use rust_decimal::{Decimal, dec};
use std::time::Duration;
use std::{future::Future, pin::Pin};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
#[allow(unused_imports)]
use tracing::{self, debug, error, info, warn};
use tracing_appender;
use tracing_subscriber::{
    self, Layer, Registry, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};
use trading_core::{
    OrderBook, Result as ClientResult,
    exchange::{Client, TEST_ENDPOINT_REST},
};
use url::Url;
use uuid::Uuid;

const ACCOUNT_NAME: &'static str = "test";
const ACCOUNT_INFO_PATH: &'static str = "./test/test_account_info.csv";
const LOG_PATH: &'static str = "./logs";

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const TEST_ENDPOINT_WS: &'static str = "wss://fstream.binancefuture.com/ws";
#[allow(dead_code)]
const ENDPOINT_WS: &'static str = "wss://fstream.binance.com/ws";

#[tokio::main]
async fn main() -> Result<()> {
    // Configure tracing subscriber
    let file_appender = tracing_appender::rolling::daily(LOG_PATH, "test.log");
    let (nb_file_writer, _guard1) = tracing_appender::non_blocking(file_appender);
    let (nb_console_writer, _guard2) = tracing_appender::non_blocking(std::io::stdout());
    let file_layer = fmt::layer()
        .with_writer(nb_file_writer)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_ansi(false)
        .with_filter(LevelFilter::INFO);

    let console_layer = fmt::layer()
        .with_writer(nb_console_writer)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .compact()
        .pretty()
        .with_filter(LevelFilter::INFO);

    Registry::default()
        .with(console_layer)
        .with(file_layer)
        .init();

    // build shared http client
    let http = reqwest::Client::builder()
        .tcp_nodelay(true)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .pool_idle_timeout(IDLE_TIMEOUT)
        .build()?;

    let client = Client::new(ACCOUNT_NAME, ACCOUNT_INFO_PATH, SOLUSDT, http.clone())?;

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
    let (acct_cmd_tx, acct_cmd_rx) = mpsc::channel(32);
    let (acct_evt_tx, mut acct_evt_rx) = mpsc::channel(1024);

    let ws = WsSession::market(url, ws_config, cmd_rx, evt_tx);
    let user_ws = WsSession::account(user_url, ws_config.clone(), acct_cmd_rx, acct_evt_tx);

    ws.spawn();
    user_ws.spawn();

    cmd_tx
        .send(StreamCommand::Subscribe(vec![StreamSpec::Depth {
            symbol: SOLUSDT,
            levels: None,
            interval_ms: None,
        }]))
        .await?;

    acct_cmd_tx
        .send(StreamCommand::Subscribe(vec![StreamSpec::TradeLite]))
        .await?;

    info!("----------INITILIAZATION FINISHED----------");

    let mut depth_buffer: Vec<Depth> = Vec::with_capacity(8);
    let mut order_book: Option<OrderBook> = None;
    let mut snapshot_fut = snapshot_task(SOLUSDT, http.clone(), 1000, Duration::from_millis(1000));
    let mut depth_counter: u64 = 0;
    let mut keepalive_interval = tokio::time::interval(Duration::from_secs(50 * 60));
    let mut order_interval = tokio::time::interval(Duration::from_secs(10));
    let mut last_client_order_id: Option<Uuid> = None;

    // MAIN EVENT LOOP
    loop {
        tokio::select! {
            // Send keep alive request
            _ = keepalive_interval.tick() => {
                match client.keepalive_listen_key().await {
                    Ok(key) => info!(listen_key=%key, "Listen key keepalive sent"),
                    Err(err) => warn!(%err, "Listen key keepalive failed"),
                }
            }

            // Account stream received
            Some(user_event) = acct_evt_rx.recv() => match user_event {
                AccountStream::TradeLite(_) => {},
                AccountStream::Raw(_) => {},
            },

            // Market stream received
            Some(event) = evt_rx.recv() => match event {
                MarketStream::Depth(depth) => {
                    depth_counter += 1;
                    if let Some(ob) = order_book.as_mut() {
                        if (depth.last_final_update_id..=depth.final_update_id).contains(&ob.last_update_id) {
                            // TODO: recheck the gap-detection logic here
                            ob.extend(depth);
                            if depth_counter % 100 == 0 {
                                info!(
                                    last_update_id = %ob.last_update_id,
                                    bids = %ob.bids.len(),
                                    asks = %ob.asks.len(),
                                    "Order book depth checkpoint"
                                );
                            }
                        } else {
                            warn!(
                                last_final_update_id = %depth.last_final_update_id,
                                first_update_id = %depth.first_update_id,
                                final_update_id = %depth.final_update_id,
                                "Gap detected in depth updates"
                            );
                            order_book = None;
                            snapshot_fut = snapshot_task(SOLUSDT, http.clone(), 1000, Duration::from_millis(1000));
                        }
                    } else {
                        // Order book not constructed yet
                        depth_buffer.push(depth);
                        info!(buffer_size=%&depth_buffer.len(), "Depth pushed to buffer");
                    }
                }
                // TODO: we still construct the events even if they are immediately dropped
                MarketStream::AggTrade(_) | MarketStream::Trade(_) | MarketStream::Raw(_) => {},
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
                info!(last_update_id=%ob.last_update_id, "Order book ready");
                order_book = Some(ob);
            },

            // cancel stale order and open new order
            _ = order_interval.tick(), if order_book.is_some() => {
                if let Some(prev_id) = last_client_order_id {
                    match client.cancel_order(prev_id).await {
                        Ok(cancel) => {
                            info!(
                                symbol=%cancel.symbol,
                                price=%cancel.price,
                                client_order_id=%cancel.client_order_id,
                                order_id=%cancel.order_id,
                                "Cancel order ACK"
                            );
                        }
                        Err(err) => {
                            warn!(%err, "Cancel order failed");
                        }
                    }
                }

                let order_request = create_order();
                match client.open_order(order_request).await {
                    Ok(success) => {
                        last_client_order_id = Some(success.client_order_id);
                        info!(
                            symbol=%success.symbol,
                            price=%success.price,
                            client_order_id=%success.client_order_id,
                            order_id=%success.order_id,
                            "Open order ACK"
                        );
                    }
                    Err(err) => {
                        warn!(%err, "Open order failed");
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
) -> Pin<Box<dyn Future<Output = ClientResult<OrderBook>> + Send>> {
    Box::pin(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        OrderBook::from_snapshot(symbol, depth, TEST_ENDPOINT_REST, http).await
    })
}

fn create_order() -> RequestOpen {
    let ts = std::cmp::max(Utc::now().timestamp_millis() % 10000, 6969);

    RequestOpen::new(
        Side::Buy,
        Decimal::new(ts, 2),
        dec!(1),
        OrderKind::Limit,
        TimeInForce::GoodUntilCancel,
        None,
    )
}
