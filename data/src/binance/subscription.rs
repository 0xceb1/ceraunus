use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
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
        levels: u16,
        interval_ms: u16,
    },
}

impl StreamSpec {
    fn as_param(&self) -> String {
        match self {
            StreamSpec::Depth {
                symbol,
                levels,
                interval_ms,
            } => format!(
                "{}@depth{}@{}ms",
                symbol.as_ref().to_lowercase(),
                levels,
                interval_ms
            ),
        }
    }
}

pub enum Command {
    Subscribe(Vec<StreamSpec>),
    Unsubscribe(Vec<StreamSpec>),
    Shutdown,
}

pub enum Event {
    Depth(BookDepth),
    Raw(Utf8Bytes), // fallback
}

#[derive(Debug)]
pub struct WsSession {
    endpoint: Url,
    config: WebSocketConfig,
    active: HashSet<StreamSpec>,
    next_id: u64,
    cmd_rx: mpsc::Receiver<Command>,
    evt_tx: mpsc::Sender<Event>,
}

impl WsSession {
    pub fn new(
        endpoint: Url,
        config: WebSocketConfig,
        cmd_rx: mpsc::Receiver<Command>,
        evt_tx: mpsc::Sender<Event>,
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
                                if let Ok(depth) = serde_json::from_str::<BookDepth>(&txt) {
                                    let _ = session.evt_tx.send(Event::Depth(depth)).await;
                                } else {
                                    let _ = session.evt_tx.send(Event::Raw(txt)).await;
                                }
                            }
                            Some(Ok(Message::Binary(bin))) => {
                                if let Ok(depth) = serde_json::from_slice::<BookDepth>(&bin) {
                                    let _ = session.evt_tx.send(Event::Depth(depth)).await;
                                } else if let Ok(txt) = Utf8Bytes::try_from(bin) {
                                    let _ = session.evt_tx.send(Event::Raw(txt)).await;
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
                            Some(Command::Subscribe(specs)) => {
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                session.active.extend(specs);
                                let cmd = WsSubscriptionCommand::new("SUBSCRIBE", params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(Command::Unsubscribe(specs)) => {
                                for spec in &specs {
                                    session.active.remove(spec);
                                }
                                let params: Vec<String> = specs.iter().map(StreamSpec::as_param).collect();
                                let cmd = WsSubscriptionCommand::new("UNSUBSCRIBE", params, session.next_id);
                                session.next_id += 1;
                                let _ = ws_sink.send(Message::Text(cmd.to_string().into())).await;
                            }
                            Some(Command::Shutdown) => break,
                            None => break,
                        }
                    }
                }
            }
        })
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Deserialize, Serialize)]
#[serde(from = "(Decimal, Decimal)")]
pub struct Level {
    pub price: Decimal,
    pub amount: Decimal,
}

impl From<(Decimal, Decimal)> for Level {
    fn from((price, amount): (Decimal, Decimal)) -> Self {
        Self { price, amount }
    }
}

/// Payload model for a depth update
/// https://developers.binance.com/docs/zh-CN/derivatives/usds-margined-futures/websocket-market-streams/Mark-Price-Stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookDepth {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: Symbol,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<Level>,
    #[serde(rename = "a")]
    asks: Vec<Level>,
}
