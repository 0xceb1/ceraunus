use crate::order::*;
use chrono::{DateTime, Utc};

use derive_more::Display;
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Deserialize, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
pub enum ExecutionType {
    New,
    Canceled,
    Calculated,
    Expired,
    Trade,
    Amendment,
}

/// Top-level payload model for verbose `ORDER_TRADE_UPDATE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Order-Update
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "o")]
    update: OrderTradeUpdate,
}

impl OrderTradeUpdateEvent {
    pub fn event_time(&self) -> DateTime<Utc> {
        self.event_time
    }

    pub fn transaction_time(&self) -> DateTime<Utc> {
        self.transaction_time
    }

    pub fn update(&self) -> &OrderTradeUpdate {
        &self.update
    }

    pub fn symbol(&self) -> Symbol {
        self.update.symbol
    }

    pub fn client_order_id(&self) -> Uuid {
        self.update.client_order_id
    }

    pub fn side(&self) -> Side {
        self.update.side
    }

    pub fn order_kind(&self) -> OrderKind {
        self.update.order_kind
    }

    pub fn time_in_force(&self) -> TimeInForce {
        self.update.time_in_force
    }

    pub fn orig_qty(&self) -> Decimal {
        self.update.orig_qty
    }

    pub fn orig_price(&self) -> Decimal {
        self.update.orig_price
    }

    pub fn avg_price(&self) -> Decimal {
        self.update.avg_price
    }

    pub fn exec_type(&self) -> ExecutionType {
        self.update.exec_type
    }

    pub fn order_status(&self) -> OrderStatus {
        self.update.order_status
    }

    pub fn order_id(&self) -> u64 {
        self.update.order_id
    }

    pub fn last_filled_qty(&self) -> Decimal {
        self.update.last_filled_qty
    }

    pub fn last_filled_amount(&self) -> Decimal {
        self.update.last_filled_price * self.update.last_filled_qty
    }

    pub fn filled_qty(&self) -> Decimal {
        self.update.filled_qty
    }

    pub fn last_filled_price(&self) -> Decimal {
        self.update.last_filled_price
    }

    pub fn commission(&self) -> Decimal {
        self.update.commission
    }

    pub fn trade_time(&self) -> DateTime<Utc> {
        self.update.trade_time
    }

    pub fn trade_id(&self) -> u64 {
        self.update.trade_id
    }

    pub fn is_maker(&self) -> bool {
        self.update.is_maker
    }

    pub fn realized_profit(&self) -> Decimal {
        self.update.realized_profit
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "c")]
    client_order_id: Uuid,

    #[serde(rename = "S")]
    side: Side,

    #[serde(rename = "o")]
    order_kind: OrderKind,

    #[serde(rename = "f")]
    time_in_force: TimeInForce,

    #[serde(rename = "q")]
    orig_qty: Decimal,

    #[serde(rename = "p")]
    orig_price: Decimal,

    #[serde(rename = "ap")]
    avg_price: Decimal,

    #[serde(rename = "x")]
    exec_type: ExecutionType,

    #[serde(rename = "X")]
    order_status: OrderStatus,

    #[serde(rename = "i")]
    order_id: u64,

    #[serde(rename = "l")]
    last_filled_qty: Decimal,

    #[serde(rename = "z")]
    filled_qty: Decimal,

    #[serde(rename = "L")]
    last_filled_price: Decimal,

    #[serde(rename = "n")]
    // WARN: in USDT
    commission: Decimal,

    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    trade_time: DateTime<Utc>,

    #[serde(rename = "t")]
    trade_id: u64,

    #[serde(rename = "m")]
    is_maker: bool,

    #[serde(rename = "rp")]
    realized_profit: Decimal,
}

/// Payload model for `TRADE_LITE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Trade-Lite
#[derive(Debug, Clone, Copy, Deserialize)]
#[allow(dead_code)]
pub struct TradeLite {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "s")]
    symbol: Symbol,
    #[serde(rename = "q")]
    orig_qty: Decimal,
    #[serde(rename = "p")]
    orig_price: Decimal,
    #[serde(rename = "m")]
    is_makter: bool,
    #[serde(rename = "c")]
    client_order_id: Uuid,
    #[serde(rename = "S")]
    side: Side,
    #[serde(rename = "L")]
    last_filled_price: Decimal,
    #[serde(rename = "l")]
    last_filled_qty: Decimal,

    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "i")]
    order_id: u64,
}

impl TradeLite {
    pub fn log(&self) {
        tracing::info!(
            client_order_id=%self.client_order_id,
            symbol=%self.symbol,
            orig_price=%self.orig_price,
            orig_qty=%self.orig_qty,
            last_filled_price=%self.last_filled_price,
            last_filled_qty=%self.last_filled_qty,
            is_makter=%self.is_makter,
            side=%self.side,
            "TradeLite event received"
        );
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[display(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountEventType {
    Deposit,
    Withdraw,
    Order,
    FundingFee,
    WithdrawReject,
    Adjustment,
    InsuranceClear,
    AdminDeposit,
    AdminWithdraw,
    MarginTransfer,
    MarginTypeChange,
    AssetTransfer,
    OptionsPremiumFee,
    OptionsSettleProfit,
    AutoExchange,
    CoinSwapDeposit,
    CoinSwapWithdraw,
}

/// Top-level payload model for `ACCOUNT_UPDATE` stream
/// https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Event-Balance-and-Position-Update
#[derive(Debug, Clone, Deserialize)]
pub struct AccountUpdateEvent {
    #[serde(rename = "E", with = "chrono::serde::ts_milliseconds")]
    event_time: DateTime<Utc>,
    #[serde(rename = "T", with = "chrono::serde::ts_milliseconds")]
    transaction_time: DateTime<Utc>,
    #[serde(rename = "a")]
    update: AccountUpdate,
}

impl AccountUpdateEvent {
    pub fn event_time(&self) -> DateTime<Utc> {
        self.event_time
    }

    pub fn transaction_time(&self) -> DateTime<Utc> {
        self.transaction_time
    }

    pub fn update(&self) -> &AccountUpdate {
        &self.update
    }

    pub fn reason(&self) -> AccountEventType {
        self.update.reason
    }

    pub fn balances(&self) -> &[BalanceUpdate] {
        &self.update.balances
    }

    pub fn positions(&self) -> &[PositionUpdate] {
        &self.update.positions
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountUpdate {
    #[serde(rename = "m")]
    reason: AccountEventType,

    #[serde(rename = "B", default)]
    balances: Vec<BalanceUpdate>,

    #[serde(rename = "P", default)]
    positions: Vec<PositionUpdate>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[allow(dead_code)]
pub struct BalanceUpdate {
    #[serde(rename = "a")]
    asset: Asset,

    #[serde(rename = "wb")]
    wallet_balance: Decimal,

    #[serde(rename = "cw")]
    cross_wallet_balance: Decimal,

    #[serde(rename = "bc")]
    balance_change: Decimal, // Balance change except PnL and commission
}

#[derive(Debug, Clone, Copy, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
#[display(rename_all = "lowercase")]
pub enum MarginType {
    Isolated,
    Cross,
}

#[derive(Debug, Clone, Copy, Deserialize, Display)]
#[serde(rename_all = "UPPERCASE")]
#[display(rename_all = "UPPERCASE")]
pub enum PositionSide {
    Both,
    Long,
    Short,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[allow(dead_code)]
pub struct PositionUpdate {
    #[serde(rename = "s")]
    symbol: Symbol,

    #[serde(rename = "pa")]
    position_amount: Decimal,

    #[serde(rename = "ep")]
    entry_price: Decimal,

    #[serde(rename = "bep")]
    breakeven_price: Decimal,

    #[serde(rename = "cr")]
    accumulated_realized: Decimal, // (Pre-fee) Accumulated realized

    #[serde(rename = "up")]
    unrealized_pnl: Decimal,

    #[serde(rename = "mt")]
    margin_type: MarginType,

    #[serde(rename = "iw")]
    isolated_wallet: Decimal,

    #[serde(rename = "ps")]
    position_side: PositionSide,
}
