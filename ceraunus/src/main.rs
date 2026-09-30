// std
use std::sync::Arc;
use std::time::Duration;

// external crates
use anyhow::Result;
use chrono::Utc;
use console_subscriber::ConsoleLayer;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tracing::{error, info, warn};
use tracing_subscriber::{
    Layer, Registry, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// Internal crates
use data::{
    binance::subscription::{AccountStream, MarketStream, StreamCommand, StreamSpec, WsSession},
    config::endpoints,
    types::Symbol::SOLUSDT,
};
use trading_core::{
    OrderBook, Result as ClientResult,
    engine::State,
    exchange::Client,
    strategy::{QuoteStrategy, Strategy},
};

const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const HTTP_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const SNAPSHOT_DEPTH: u16 = 1000;
const STALE_ORDER_THRESHOLD: chrono::Duration = chrono::Duration::seconds(30);

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

    let mkt_url = Url::parse(&format!("{}/ws", endpoints::WS_PUBLIC))?;
    let acct_url = Url::parse(&format!(
        "{}/ws?listenKey={}",
        endpoints::WS_PRIVATE,
        listen_key
    ))?;

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

    info!("----------INITIALIZATION FINISHED----------");

    let mut state: State = State::new(SOLUSDT);

    let mut snapshot = Box::pin(fetch_snapshot(http.clone(), SNAPSHOT_DEPTH));
    let mut keepalive_interval = tokio::time::interval(Duration::from_secs(50 * 60));
    let mut send_order_interval = tokio::time::interval(Duration::from_secs(10));
    let mut cancel_order_interval = tokio::time::interval(Duration::from_secs(60));
    let mut report_state_interval = tokio::time::interval(Duration::from_secs(60));

    // MAIN EVENT LOOP
    loop {
        tokio::select! {
            biased;

            Some(event) = evt_rx.recv() => match event {
                MarketStream::Depth(depth) => {
                    if state.on_depth_received(depth) {
                        snapshot.set(fetch_snapshot(http.clone(), SNAPSHOT_DEPTH));
                    }
                }
                MarketStream::BookTicker(book_ticker) => state.on_book_ticker_received(book_ticker),
                MarketStream::AggTrade(_) | MarketStream::Trade(_) | MarketStream::Raw(_) => {}
            },

            Some(event) = acct_evt_rx.recv() => on_account_event(&mut state, event),

            _ = report_state_interval.tick() => report_state(&state),

            _ = send_order_interval.tick(), if state.book.is_ready() => send_quotes(&mut state, &client),

            _ = cancel_order_interval.tick() => cancel_stale_orders(&state, &client),

            snapshot_res = &mut snapshot, if !state.book.is_ready() => {
                if state.book.on_snapshot_received(snapshot_res?) {
                    snapshot.set(fetch_snapshot(http.clone(), SNAPSHOT_DEPTH));
                }
            }

            _ = keepalive_interval.tick() => keepalive_listen_key(&client),
        }
    }
}

fn on_account_event(state: &mut State, event: AccountStream) {
    match event {
        AccountStream::OrderTradeUpdate(update_event) => {
            if let Err(err) = state.on_update_received(&update_event) {
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
    }
}

fn send_quotes(state: &mut State, client: &Arc<Client>) {
    let quotes = QuoteStrategy::generate_quotes(SOLUSDT, state);
    state.register_orders(&quotes);
    let client = Arc::clone(client);
    tokio::spawn(async move {
        let results = client.open_orders(&quotes).await;

        for result in results {
            match result {
                Ok(success) => success.log("Open order ACK"),
                Err(err) => {
                    // TODO: complete the order
                    warn!(%err, "Open order failed");
                }
            }
        }
    });
}

fn cancel_stale_orders(state: &State, client: &Arc<Client>) {
    for client_order_id in state.stale_order_ids(STALE_ORDER_THRESHOLD) {
        let client = Arc::clone(client);
        tokio::spawn(async move {
            match client.cancel_order(SOLUSDT, client_order_id).await {
                Ok(cancel) => cancel.log("Cancel stale order ACK"),
                Err(err) => error!(%err, %client_order_id, "Cancel stale order failed"),
            }
        });
    }
}

fn report_state(state: &State) {
    info!(
        elapsed = %(Utc::now() - state.start_time),
        turnover = %state.turnover,
        curr_pos = %state.get_position(),
        exec_pnl = %state.pnl.execution_pnl,
        unrealized_pnl = %state.pnl.unrealized_pnl,
        realized_pnl = %state.pnl.realized_pnl,
        ob = ?state.book.order_book().map(|ob| ob.show(5)),
        ob_bids = state.book.order_book().map_or(0, |ob| ob.bids.len()),
        ob_asks = state.book.order_book().map_or(0, |ob| ob.asks.len()),
        "Trading Summary"
    );
}

fn keepalive_listen_key(client: &Arc<Client>) {
    let client = Arc::clone(client);
    tokio::spawn(async move {
        match client.keepalive_listen_key().await {
            Ok(key) => info!(listen_key = %key, "Listen key keepalive sent"),
            Err(err) => error!(%err, "Listen key keepalive failed"),
        }
    });
}

/// Waits briefly so depth updates are buffered before the snapshot is taken
async fn fetch_snapshot(http: reqwest::Client, depth: u16) -> ClientResult<OrderBook> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    OrderBook::from_snapshot(SOLUSDT, depth, endpoints::REST, http).await
}
