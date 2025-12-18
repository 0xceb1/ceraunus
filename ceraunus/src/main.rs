// std
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

// external crates
use anyhow::Result;
use chrono::Utc;
use console_subscriber::ConsoleLayer;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tracing::{error, info, warn};
use tracing_subscriber::{
    Layer, Registry, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

// Internal crates
use data::{
    binance::market::Depth,
    binance::subscription::{AccountStream, MarketStream, StreamCommand, StreamSpec, WsSession},
    order::{Symbol, Symbol::SOLUSDT},
};
use trading_core::{
    OrderBook, Result as ClientResult,
    engine::State,
    exchange::Client,
    strategy::{QuoteStrategy, Strategy},
};

const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const STALE_ORDER_THRESHOLD: chrono::Duration = chrono::Duration::seconds(30);

#[derive(Debug)]
enum Event {
    Account(AccountStream),
    Market(MarketStream),
    SnapshotDone(ClientResult<OrderBook>),
    SendOrderTick,
    CancelOrderTick,
    ReportStateTick,
    KeepaliveTick,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg_path = std::env::var("CERAUNUS_CONFIG")
        .unwrap_or_else(|_| "./config/datacenter-config.toml".to_string());
    let cfg = data::config::DataCenterConfig::load(&cfg_path)?;

    if cfg.logging.file_log {
        std::fs::create_dir_all(&cfg.logging.file.dir)?;
    }

    // Configure tracing subscriber
    let file_appender =
        tracing_appender::rolling::daily(&cfg.logging.file.dir, &cfg.logging.file.name);
    let (nb_file_writer, _guard1) = tracing_appender::non_blocking(file_appender);
    let (nb_console_writer, _guard2) = tracing_appender::non_blocking(std::io::stdout());

    let file_filter = cfg
        .logging
        .file
        .level
        .parse::<LevelFilter>()
        .unwrap_or(LevelFilter::INFO);
    let console_filter = cfg
        .logging
        .console
        .level
        .parse::<LevelFilter>()
        .unwrap_or(LevelFilter::INFO);

    let file_layer = fmt::layer()
        .with_writer(nb_file_writer)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_ansi(false)
        .with_filter(file_filter);

    let stdout_layer = fmt::layer()
        .with_writer(nb_console_writer)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .compact()
        // .pretty()
        .with_filter(console_filter);

    // Tokio console layer (enable/configure via env vars; see tokio-console docs)
    let tokio_console_layer = ConsoleLayer::builder().with_default_env().spawn();

    Registry::default()
        .with(stdout_layer)
        .with(file_layer)
        .with(tokio_console_layer)
        .init();

    // build shared http client
    let http = reqwest::Client::builder()
        .tcp_nodelay(true)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .pool_idle_timeout(IDLE_TIMEOUT)
        .build()?;

    let client = Arc::new(Client::from_config(&cfg, http.clone())?);

    let listen_key = client.get_listen_key().await?;

    let ws_url = match cfg.account.environment {
        data::config::Environment::Production => &cfg.exchange.ws.endpoints.production,
        data::config::Environment::Testnet => &cfg.exchange.ws.endpoints.testnet,
    };

    let rest_url = match cfg.account.environment {
        data::config::Environment::Production => &cfg.exchange.rest.endpoints.production,
        data::config::Environment::Testnet => &cfg.exchange.rest.endpoints.testnet,
    };

    let mkt_url = Url::parse(ws_url)?;
    let acct_url = Url::parse(&format!("{}/{}", ws_url, listen_key))?;

    let ws_config = WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(256 * 1024)
        .max_message_size(Some(512 * 1024))
        .max_frame_size(Some(256 * 1024));

    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (evt_tx, mut evt_rx) = mpsc::channel(1024);
    let (acct_cmd_tx, acct_cmd_rx) = mpsc::channel(32);
    let (acct_evt_tx, mut acct_evt_rx) = mpsc::channel(1024);

    let ws = WsSession::market(mkt_url, ws_config, cmd_rx, evt_tx);
    let acct_ws = WsSession::account(acct_url, ws_config, acct_cmd_rx, acct_evt_tx);

    ws.spawn_named("ws.market.session");
    acct_ws.spawn_named("ws.account.session");

    cmd_tx
        .send(StreamCommand::Subscribe(vec![
            StreamSpec::Depth {
                symbol: SOLUSDT,
                levels: None,
                interval_ms: None,
            },
            StreamSpec::BookTicker { symbol: SOLUSDT },
        ]))
        .await?;

    acct_cmd_tx
        .send(StreamCommand::Subscribe(vec![
            StreamSpec::OrderTradeUpdate,
            // StreamSpec::TradeLite,
        ]))
        .await?;

    info!("----------INITILIAZATION FINISHED----------");

    let mut state: State = State::new();

    let mut depth_buffer: Vec<Depth> = Vec::with_capacity(8);
    let mut snapshot_fut = snapshot_task(
        SOLUSDT,
        http.clone(),
        1000,
        Duration::from_millis(1000),
        rest_url.clone(),
    );
    let mut keepalive_interval = tokio::time::interval(Duration::from_secs(50 * 60));
    let mut send_order_interval = tokio::time::interval(Duration::from_secs(10));
    let mut cancel_order_interval = tokio::time::interval(Duration::from_secs(60));
    let mut report_state_interval = tokio::time::interval(Duration::from_secs(60));

    // MAIN EVENT LOOP
    loop {
        let event = tokio::select! {
            biased;

            Some(event) = evt_rx.recv() => Event::Market(event),

            Some(acct_event) = acct_evt_rx.recv() => Event::Account(acct_event),

            _ = report_state_interval.tick() => Event::ReportStateTick,

            _ = send_order_interval.tick(), if state.has_order_book(SOLUSDT) => Event::SendOrderTick,

            _ = cancel_order_interval.tick() => Event::CancelOrderTick,

            snapshot_res = &mut snapshot_fut, if !state.has_order_book(SOLUSDT) => Event::SnapshotDone(snapshot_res),

            _ = keepalive_interval.tick() => Event::KeepaliveTick,
        };

        match event {
            Event::Account(acct_event) => match acct_event {
                AccountStream::OrderTradeUpdate(update_event) => {
                    if let Err(err) = state.on_update_received(update_event) {
                        error!(
                            %err,
                            symbol = %update_event.symbol(),
                            order_id = %update_event.order_id(),
                            client_order_id = %update_event.client_order_id(),
                            exec_type = %update_event.exec_type(),
                            order_status = %update_event.order_status(),
                            "Failed to process order update"
                        );
                    }
                }
                AccountStream::TradeLite(trade_lite) => {
                    trade_lite.log();
                }
                AccountStream::AccountUpdate(update_event) => {
                    info!(
                        reason = %update_event.reason(),
                        "Account update received"
                    );
                }
                AccountStream::Raw(_) => {}
            },

            Event::Market(event) => match event {
                MarketStream::Depth(depth) => {
                    if let Some(ob) = &mut state.order_books[SOLUSDT] {
                        if (depth.last_final_update_id()..=depth.final_update_id())
                            .contains(&ob.last_update_id())
                        {
                            // TODO: recheck the gap-detection logic here
                            ob.extend(depth);
                        } else {
                            warn!(
                                last_final_update_id = %depth.last_final_update_id(),
                                first_update_id = %depth.first_update_id(),
                                final_update_id = %depth.final_update_id(),
                                "Gap detected in depth updates"
                            );
                            state.remove_order_book(SOLUSDT);
                            snapshot_fut = snapshot_task(
                                SOLUSDT,
                                http.clone(),
                                1000,
                                Duration::from_millis(1000),
                                rest_url.clone(),
                            );
                        }
                    } else {
                        // Order book not constructed yet
                        depth_buffer.push(depth);
                        info!(buffer_size=%&depth_buffer.len(), "Depth pushed to buffer");
                    }
                }
                MarketStream::BookTicker(book_ticker) => {
                    state.on_book_ticker_received(book_ticker);
                }
                // TODO: we still construct the events even if they are immediately dropped
                MarketStream::AggTrade(_) | MarketStream::Trade(_) | MarketStream::Raw(_) => {}
            },

            Event::SnapshotDone(snapshot_res) => {
                let mut ob = snapshot_res?;

                for depth in depth_buffer.drain(..) {
                    if depth.final_update_id() < ob.last_update_id() {
                        continue; // too old
                    } else {
                        // TODO: we don't check U <= lastUpdateId AND u >= lastUpdateId here
                        ob.extend(depth);
                    }
                }
                info!(last_update_id=%ob.last_update_id(), "Order book ready");
                state.order_books[SOLUSDT] = Some(ob);
            }

            Event::CancelOrderTick => {
                let stale_ids = state.stale_order_ids(STALE_ORDER_THRESHOLD);

                for stale_id in stale_ids {
                    let client = Arc::clone(&client);
                    tokio::spawn(async move {
                        match client.cancel_order(SOLUSDT, stale_id).await {
                            Ok(cancel) => {
                                info!(
                                    symbol=%cancel.symbol(),
                                    price=%cancel.price(),
                                    client_order_id=%cancel.client_order_id(),
                                    order_id=%cancel.order_id(),
                                    "Cancel stale order ACK"
                                );
                            }
                            Err(err) => {
                                error!(%err, %stale_id, "Cancel stale order failed");
                            }
                        }
                    });
                }
            }

            Event::SendOrderTick => {
                let quotes = QuoteStrategy::generate_quotes(SOLUSDT, &state);
                state.register_orders(&quotes);
                let client = Arc::clone(&client);
                tokio::spawn(async move {
                    let results = client.open_orders(&quotes).await;

                    for result in results {
                        match result {
                            Ok(success) => info!(
                                symbol=%success.symbol(),
                                price=%success.price(),
                                client_order_id=%success.client_order_id(),
                                order_id=%success.order_id(),
                                "Open order ACK"
                            ),
                            Err(err) => {
                                // TODO: complete the order
                                warn!(%err, "Open order failed");
                            },
                        }
                    }
                });
            }

            Event::ReportStateTick => {
                info!(
                    elapsed = %(Utc::now() - state.start_time()),
                    turnover = %state.turnover(),
                    curr_pos = %state.get_position(SOLUSDT),
                    ob = ?state.order_books[SOLUSDT].as_ref().map(|ob| ob.show(5)),
                    "Trading Summary"
                );
            }

            Event::KeepaliveTick => {
                let client = Arc::clone(&client);
                tokio::spawn(async move {
                    match client.keepalive_listen_key().await {
                        Ok(key) => info!(listen_key=%key, "Listen key keepalive sent"),
                        Err(err) => error!(%err, "Listen key keepalive failed"),
                    }
                });
            }
        }
    }
}

fn snapshot_task(
    symbol: Symbol,
    http: reqwest::Client,
    depth: u16,
    delay: Duration,
    rest_endpoint: String,
) -> Pin<Box<dyn Future<Output = ClientResult<OrderBook>> + Send>> {
    Box::pin(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        OrderBook::from_snapshot(symbol, depth, &rest_endpoint, http).await
    })
}
