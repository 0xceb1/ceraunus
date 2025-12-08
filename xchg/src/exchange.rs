use data::{config::AccountConfidential, request::RequestOpen};
use std::error::Error;

pub struct Client {
    credentials: AccountConfidential,
}

impl Client {
    pub async fn open_order(request: RequestOpen) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
