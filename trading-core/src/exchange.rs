use data::{config::AccountConfidential, request::RequestOpen};
use std::error::Error;

pub struct Client {
    confidential: AccountConfidential,
}

impl Client {
    pub async fn open_order(&self, request: RequestOpen) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use data::order::{OrderKind, Side, TimeInForce};
    use rust_decimal::dec;

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_open_order() {
        let gtd = Utc::now() + Duration::minutes(10);
        let gtd = gtd.timestamp_millis() as u64;
        let order_request = RequestOpen {
            side: Side::Buy,
            price: dec!(1.0),
            quantity: dec!(0.01),
            kind: OrderKind::Limit,
            time_in_force: TimeInForce::GoodUntilDate {
                good_till_date: gtd,
            },
        };

        let confidential = AccountConfidential::from_csv("test", "../test/test_account_info.csv").expect("Cannot read confidentials from csv file.");
        let client = Client { confidential };

        let res = client.open_order(order_request).await;
        assert!(res.is_ok())
    }
}
