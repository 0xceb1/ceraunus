use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, future::Future};
use derive_more::Display;
use tokio::{select, sync::mpsc, task::JoinHandle};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        Utf8Bytes,
        protocol::{Message, WebSocketConfig},
    },
};
use tracing::warn;
use url::Url;

use crate::binance::account::{AccountUpdateEvent, OrderTradeUpdateEvent, TradeLite};
use crate::binance::market::*;
use crate::order::Symbol;

#[derive(Debug, Serialize, Clone, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
pub(crate) enum WsSubscriptionMethod {
    Subscribe,
    Unsubscribe,
}

/// Serialized control message sent to Binance to subscribe/unsubscribe streams.
#[derive(Debug, Serialize)]
pub struct WsSubscriptionCommand {
    method: WsSubscriptionMethod,
    params: Vec<String>, // stream names per Binance docs
    id: u64,
}

impl fmt::Display for WsSubscriptionCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", json)
    }
}

impl WsSubscriptionCommand {
    pub(crate) fn new(method: WsSubscriptionMethod, params: Vec<String>, id: u64) -> Self {
        Self { method, params, id }
    }
}

/// Available streams
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum StreamSpec {
    // market streams
    Depth {
        symbol: Symbol,
        levels: Option<u16>,
        interval_ms: Option<u16>,
    },
    BookTicker { symbol: Symbol, },
    AggTrade { symbol: Symbol, },
    Trade { symbol: Symbol },

    // account streams
    OrderTradeUpdate,
    TradeLite,
    AccountUpdate,
}

impl StreamSpec {
    fn as_param(&self) -> String {
        use StreamSpec as S;
        match self {
            S::Depth {
                symbol,
                levels,
                interval_ms,
            } => match (levels, interval_ms) {
                (Some(l), Some(i)) => format!("{}@depth{l}@{i}ms", symbol.as_ref().to_lowercase()),
                (Some(l), None) => format!("{}@depth{l}", symbol.as_ref().to_lowercase()),
                (None, Some(i)) => format!("{}@depth@{i}ms", symbol.as_ref().to_lowercase()),
                (None, None) => format!("{}@depth", symbol.as_ref().to_lowercase()),
            },
            S::BookTicker { symbol } => format!("{}@bookTicker", symbol.as_ref().to_lowercase()),
            S::AggTrade { symbol } => format!("{}@aggTrade", symbol.as_ref().to_lowercase()),
            S::Trade { symbol } => format!("{}@trade", symbol.as_ref().to_lowercase()),
            S::TradeLite => "TRADE_LITE".to_string(),
            S::OrderTradeUpdate => "ORDER_TRADE_UPDATE".to_string(),
            S::AccountUpdate => "ACCOUNT_UPDATE".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum StreamCommand {
    Subscribe(Vec<StreamSpec>),
    Unsubscribe(Vec<StreamSpec>),
    Shutdown,
}

pub trait ParseStream: Sized {
    fn parse(text: &str) -> Self;
}

#[derive(Debug)]
pub enum MarketStream {
    Depth(Depth),
    BookTicker(BookTicker),
    AggTrade(AggTrade),
    Trade(Trade),
    Raw(Utf8Bytes),
}

impl ParseStream for MarketStream {
    fn parse(text: &str) -> Self {
        match serde_json::from_str::<MarketPayload>(text) {
            Ok(MarketPayload::Depth(depth)) => MarketStream::Depth(depth),
            Ok(MarketPayload::BookTicker(book_ticker)) => MarketStream::BookTicker(book_ticker),
            Ok(MarketPayload::AggTrade(agg_trade)) => MarketStream::AggTrade(agg_trade),
            Ok(MarketPayload::Trade(trade)) => MarketStream::Trade(trade),
            Err(_) => {
                let stream = MarketStream::Raw(Utf8Bytes::from(text));
                warn!(?stream, "Raw market stream (unparsed)");
                stream
            }
        }
    }
}

#[derive(Debug)]
pub enum AccountStream {
    OrderTradeUpdate(OrderTradeUpdateEvent),
    TradeLite(TradeLite),
    AccountUpdate(AccountUpdateEvent),
    Raw(Utf8Bytes),
}

impl ParseStream for AccountStream {
    fn parse(text: &str) -> Self {
        match serde_json::from_str::<AccountPayload>(text) {
            Ok(AccountPayload::OrderTradeUpdate(update)) => AccountStream::OrderTradeUpdate(update),
            Ok(AccountPayload::TradeLite(trade_lite)) => AccountStream::TradeLite(trade_lite),
            Ok(AccountPayload::AccountUpdate(account_update)) => AccountStream::AccountUpdate(account_update),
            Err(_) => {
                let stream = AccountStream::Raw(Utf8Bytes::from(text));
                warn!(?stream, "Raw account stream (unparsed)");
                stream
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "e")]
enum MarketPayload {
    #[serde(rename = "depthUpdate")]
    Depth(Depth),
    #[serde(rename = "bookTicker")]
    BookTicker(BookTicker),
    #[serde(rename = "trade")]
    Trade(Trade),
    #[serde(rename = "aggTrade")]
    AggTrade(AggTrade),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "e", rename_all = "SCREAMING_SNAKE_CASE")]
enum AccountPayload {
    OrderTradeUpdate(OrderTradeUpdateEvent),
    TradeLite(TradeLite),
    AccountUpdate(AccountUpdateEvent),
}

#[derive(Debug)]
pub struct WsSession<E> {
    endpoint: Url,
    config: WebSocketConfig,
    active: HashSet<StreamSpec>,
    next_id: u64,
    cmd_rx: mpsc::Receiver<StreamCommand>,
    evt_tx: mpsc::Sender<E>,
}

impl<E> WsSession<E> {
    fn new(
        endpoint: Url,
        config: WebSocketConfig,
        cmd_rx: mpsc::Receiver<StreamCommand>,
        evt_tx: mpsc::Sender<E>,
    ) -> Self {
        Self {
            endpoint,
            config,
            active: HashSet::new(),
            next_id: 1,
            cmd_rx,
            evt_tx,
        }
    }
}

impl WsSession<MarketStream> {
    pub fn market(
        endpoint: Url,
        config: WebSocketConfig,
        cmd_rx: mpsc::Receiver<StreamCommand>,
        evt_tx: mpsc::Sender<MarketStream>,
    ) -> Self {
        Self::new(endpoint, config, cmd_rx, evt_tx)
    }
}

impl WsSession<AccountStream> {
    pub fn account(
        endpoint: Url,
        config: WebSocketConfig,
        cmd_rx: mpsc::Receiver<StreamCommand>,
        evt_tx: mpsc::Sender<AccountStream>,
    ) -> Self {
        Self::new(endpoint, config, cmd_rx, evt_tx)
    }
}

impl<E> WsSession<E>
where
    E: ParseStream + 'static + Send + Sync + fmt::Debug,
{
    fn task(self) -> impl Future<Output = ()> + Send + 'static {
        async move {
            let mut session = self;
            let Ok((ws_stream, _)) =
                connect_async_with_config(session.endpoint.as_str(), Some(session.config), true)
                    .await
            else {
                return;
            };

            let (mut ws_sink, mut ws_stream) = ws_stream.split();

            loop {
                select! {
                    // if a message is received
                    maybe_msg = ws_stream.next() => {
                        match maybe_msg {
                            Some(Ok(Message::Text(txt))) => {
                                // debug!(msg_type = "text", "text message received");
                                let event = E::parse(&txt);
                                let _ = session.evt_tx.send(event).await;
                            }
                            Some(Ok(raw)) => {
                                let msg_type = match &raw {
                                    Message::Text(_) => "text",
                                    Message::Binary(_) => "binary",
                                    Message::Ping(_) => "ping",
                                    Message::Pong(_) => "pong",
                                    Message::Close(_) => "close",
                                    Message::Frame(_) => "frame",
                                };
                                warn!(
                                    %msg_type, ?raw,
                                    "unexpected message received"
                                );
                            }
                            Some(Err(_e)) => break,
                            None => break,
                        }
                    }
                    // if a command sent
                    maybe_cmd = session.cmd_rx.recv() => {
                        use WsSubscriptionMethod as M;
                        match maybe_cmd {
                            Some(StreamCommand::Subscribe(specs)) => {
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                session.active.extend(specs);
                                let cmd = WsSubscriptionCommand::new(M::Subscribe, params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(StreamCommand::Unsubscribe(specs)) => {
                                for spec in &specs {
                                    session.active.remove(spec);
                                }
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                let cmd = WsSubscriptionCommand::new(M::Unsubscribe, params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(StreamCommand::Shutdown) => break,
                            None => break,
                        }
                    }
                }
            }
        }
    }

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(self.task())
    }

    pub fn spawn_named(self, name: &'static str) -> JoinHandle<()> {
        tokio::task::Builder::new()
            .name(name)
            .spawn(self.task())
            .expect(format!("Failed to spawn task {}", name).as_str())
    }
}
