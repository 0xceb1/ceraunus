use crate::Result;
use crate::error::{ConfigError, DataError};
use crate::order::Symbol;
use csv::Reader;
use serde::{Deserialize, Deserializer};
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct AccountConfidential {
    #[serde(rename = "account_name")]
    pub name: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(rename = "testnet", deserialize_with = "bool_from_string")]
    is_testnet: bool,
}

impl AccountConfidential {
    pub fn from_csv(name: &str, csv_path: impl AsRef<Path>) -> Result<Self> {
        let mut rdr = Reader::from_path(csv_path)?;
        for result in rdr.deserialize() {
            let record: AccountConfidential = result?;

            if record.name == name {
                return Ok(record);
            }
        }
        Err(DataError::Config(ConfigError::AccountNotFound {
            name: name.to_string(),
        }))
    }

    pub fn is_testnet(&self) -> bool {
        self.is_testnet
    }
}

fn bool_from_string<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.eq_ignore_ascii_case("true"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingFileConfig {
    pub dir: String,
    pub name: String,
    pub rolling: String,
    pub level: String,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConsoleConfig {
    pub level: String,
    pub pretty: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub file_log: bool,
    pub console_log: bool,
    pub file: LoggingFileConfig,
    pub console: LoggingConsoleConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Production,
    Testnet,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountConfig {
    pub exchange: String,
    pub environment: Environment,
    pub name: String,
    pub csv_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EndpointMap {
    pub production: String,
    pub testnet: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestConfig {
    pub endpoints: EndpointMap,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WsConfig {
    pub endpoints: EndpointMap,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    pub symbols: Vec<Symbol>,
    pub rest: RestConfig,
    pub ws: WsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataCenterConfig {
    pub logging: LoggingConfig,
    pub account: AccountConfig,
    pub exchange: ExchangeConfig,
}

impl DataCenterConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let raw = fs::read_to_string(&path).map_err(ConfigError::from)?;
        let cfg: DataCenterConfig = toml::from_str(&raw).map_err(ConfigError::from)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PATH: &'static str = "../test/test_account_info.csv";
    #[test]
    fn test_read_confidential_from_csv() {
        let test_res = AccountConfidential::from_csv("test", PATH);
        let prod_res = AccountConfidential::from_csv("prod2_r", PATH);
        let fail_res = AccountConfidential::from_csv("urmom", PATH);

        assert!(test_res.is_ok());
        assert!(prod_res.is_ok());
        assert!(fail_res.is_err());
    }
}
