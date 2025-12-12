use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, convert::TryFrom, fmt};
use strum_macros::{AsRefStr, Display, EnumString};
use tokio::{select, sync::mpsc, task::JoinHandle};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        Utf8Bytes,
        protocol::{Message, WebSocketConfig},
    },
};
use url::Url;

use crate::binance::account::TradeLite;
use crate::binance::market::*;
use crate::order::Symbol;

#[derive(Debug, Serialize, Clone, Display, AsRefStr, EnumString)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
enum WsSubscriptionMethod {
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
    pub fn new(method: &str, params: Vec<String>, id: u64) -> Self {
        Self {
            method: method.parse().expect("Check your spell!"),
            params,
            id,
        }
    }
}

/// Available streams
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum StreamSpec {
    Depth {
        symbol: Symbol,
        levels: Option<u16>,
        interval_ms: Option<u16>,
    },
    AggTrade {
        symbol: Symbol,
    },
    Trade {
        symbol: Symbol,
    },
    OrderTradeUpdate,
    TradeLite,
}

impl StreamSpec {
    fn as_param(&self) -> String {
        use StreamSpec::*;
        match self {
            Depth {
                symbol,
                levels,
                interval_ms,
            } => match (levels, interval_ms) {
                (Some(l), Some(i)) => format!("{}@depth{l}@{i}ms", symbol.as_ref().to_lowercase()),
                (Some(l), None) => format!("{}@depth{l}", symbol.as_ref().to_lowercase()),
                (None, Some(i)) => format!("{}@depth@{i}ms", symbol.as_ref().to_lowercase()),
                (None, None) => format!("{}@depth", symbol.as_ref().to_lowercase()),
            },
            AggTrade { symbol } => format!("{}@aggTrade", symbol.as_ref().to_lowercase()),
            Trade { symbol } => format!("{}@trade", symbol.as_ref().to_lowercase()),
            TradeLite => "TRADE_LITE".to_string(),
            OrderTradeUpdate => "ORDER_TRADE_UPDATE".to_string(),
        }
    }
}

pub enum StreamCommand {
    Subscribe(Vec<StreamSpec>),
    Unsubscribe(Vec<StreamSpec>),
    Shutdown,
}

pub enum MarketStream {
    // Market streams
    Depth(Depth),
    AggTrade(AggTrade),
    Trade(Trade),

    // User streams
    TradeLite(TradeLite),

    // Fallback
    Raw(Utf8Bytes),
}

#[derive(Debug)]
pub enum AccountStream {
    TradeLite(TradeLite),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "e")]
enum IncomingPayload {
    // Market streams
    #[serde(rename = "depthUpdate")]
    Depth(Depth),
    #[serde(rename = "trade")]
    Trade(Trade),
    #[serde(rename = "aggTrade")]
    AggTrade(AggTrade),

    // User streams
    #[serde(rename = "TRADE_LITE")]
    TradeLite(TradeLite),
}

#[derive(Debug)]
pub struct WsSession {
    endpoint: Url,
    config: WebSocketConfig,
    active: HashSet<StreamSpec>,
    next_id: u64,
    cmd_rx: mpsc::Receiver<StreamCommand>,
    evt_tx: mpsc::Sender<MarketStream>,
}

impl WsSession {
    pub fn new(
        endpoint: Url,
        config: WebSocketConfig,
        cmd_rx: mpsc::Receiver<StreamCommand>,
        evt_tx: mpsc::Sender<MarketStream>,
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

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
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
                                match serde_json::from_str::<IncomingPayload>(&txt) {
                                    Ok(IncomingPayload::Depth(depth)) => {
                                        let _ = session.evt_tx.send(MarketStream::Depth(depth)).await;
                                    }
                                    Ok(IncomingPayload::AggTrade(agg_trade)) => {
                                        let _ = session.evt_tx.send(MarketStream::AggTrade(agg_trade)).await;
                                    }
                                    Ok(IncomingPayload::Trade(trade)) => {
                                        let _ = session.evt_tx.send(MarketStream::Trade(trade)).await;
                                    }
                                    Ok(IncomingPayload::TradeLite(trade_lite)) => {
                                        let _ = session.evt_tx.send(MarketStream::TradeLite(trade_lite)).await;
                                    }
                                    Err(_) => {
                                        let _ = session.evt_tx.send(MarketStream::Raw(txt)).await;
                                    }
                                }
                            }
                            Some(Ok(Message::Binary(bin))) => {
                                match serde_json::from_slice::<IncomingPayload>(&bin) {
                                    Ok(IncomingPayload::Depth(depth)) => {
                                        let _ = session.evt_tx.send(MarketStream::Depth(depth)).await;
                                    }
                                    Ok(IncomingPayload::AggTrade(agg_trade)) => {
                                        let _ = session.evt_tx.send(MarketStream::AggTrade(agg_trade)).await;
                                    }
                                    Ok(IncomingPayload::Trade(trade)) => {
                                        let _ = session.evt_tx.send(MarketStream::Trade(trade)).await;
                                    }
                                    Ok(IncomingPayload::TradeLite(trade_lite)) => {
                                        let _ = session.evt_tx.send(MarketStream::TradeLite(trade_lite)).await;
                                    }
                                    Err(_) => {
                                        if let Ok(txt) = Utf8Bytes::try_from(bin) {
                                            let _ = session.evt_tx.send(MarketStream::Raw(txt)).await;
                                        }
                                    }
                                }
                            }
                            Some(Ok(_)) => {}
                            Some(Err(_e)) => break,
                            None => break,
                        }
                    }
                    // if a command sent
                    maybe_cmd = session.cmd_rx.recv() => {
                        match maybe_cmd {
                            Some(StreamCommand::Subscribe(specs)) => {
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                session.active.extend(specs);
                                let cmd = WsSubscriptionCommand::new("SUBSCRIBE", params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(StreamCommand::Unsubscribe(specs)) => {
                                for spec in &specs {
                                    session.active.remove(spec);
                                }
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                let cmd = WsSubscriptionCommand::new("UNSUBSCRIBE", params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(StreamCommand::Shutdown) => break,
                            None => break,
                        }
                    }
                }
            }
        })
    }
}
