use csv::Reader;
use serde::{Deserialize, Deserializer};
use std::error::Error;
use std::path::Path;

fn bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.eq_ignore_ascii_case("true"))
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct AccountConfidential {
    #[serde(rename = "account_name")]
    pub name: String,
    pub api_key: String,
    pub api_secret: String,
    #[serde(rename = "testnet", deserialize_with = "bool_from_string")]
    pub is_testnet: bool,
}

impl AccountConfidential {
    pub fn from_csv(name: &str, csv_path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let mut rdr = Reader::from_path(csv_path)?;
        for result in rdr.deserialize() {
            let record: AccountConfidential = result?;

            if record.name == name {
                return Ok(record);
            }
        }
        Err(format!("Account with name '{}' not found", name).into())
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
