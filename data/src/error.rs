use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    WebSocket(SocketError)
}

impl From<csv::Error> for DataError {
    fn from(err: csv::Error) -> Self {
        DataError::Config(ConfigError::from(err))
    }
}

/// All errors related to config loading
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),

    #[error("account '{name}' not found")]
    AccountNotFound { name: String },
}

/// Websocket connection error
#[derive(Debug, Error)]
pub enum SocketError {
    #[error("")]
    InvalidListenKey,
}

pub type Error = DataError;
pub type Result<T> = std::result::Result<T, DataError>;
