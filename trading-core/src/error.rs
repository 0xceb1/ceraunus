use data::DataError;
use hmac::digest::InvalidLength as HmacInvalidLength;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("rate limited: {status} body {body}")]
    RateLimit { status: StatusCode, body: String },

    #[error("balance insufficient: {status} body {body}")]
    BalanceInsufficient { status: StatusCode, body: String },

    #[error("order rejected: {status} body {body}")]
    OrderRejected { status: StatusCode, body: String },

    #[error("exchange error: {status} body {body}")]
    Unknown { status: StatusCode, body: String },
}

#[derive(Debug, Error)]
pub enum ConnectivityError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Message encode/decode error
#[derive(Debug, Error)]
pub enum MessageCodecError {
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("missing field: {0}")]
    MissingField(&'static str),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("urlencode error: {0}")]
    SerdeUrl(#[from] serde_urlencoded::ser::Error),

    #[error("signing error: {0}")]
    Hmac(#[from] HmacInvalidLength),
}

#[derive(Debug, Error)]
pub enum TradingCoreError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Connectivity(#[from] ConnectivityError),

    #[error(transparent)]
    MessageCodec(#[from] MessageCodecError),

    #[error(transparent)]
    Data(#[from] DataError),

    #[error("client initialization failed: {0}")]
    ClientInitialization(String),

    #[error("SOMETHING VERY BAD HAPPENNED :( {0}")]
    Unrecoverable(String),
}

impl From<reqwest::Error> for TradingCoreError {
    fn from(err: reqwest::Error) -> Self {
        TradingCoreError::Connectivity(ConnectivityError::Network(err))
    }
}

impl From<serde_json::Error> for TradingCoreError {
    fn from(err: serde_json::Error) -> Self {
        TradingCoreError::MessageCodec(MessageCodecError::Serde(err))
    }
}

pub type Error = TradingCoreError;
pub type Result<T> = std::result::Result<T, TradingCoreError>;
