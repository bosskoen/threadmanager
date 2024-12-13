use std::time::Duration;

use reqwest::Url;
use retry::{retry, delay::Fixed};

pub use custom_types::*;

mod custom_types;

pub fn get_data(address : &str, retrys: usize, timeout: Duration) -> Result<String, WebServiceError>{
    Url::parse(address )?;
    let client = reqwest::blocking::Client::builder().timeout(timeout).build()?;
    let response = retry( Fixed::from_millis(100).take(retrys) ,|| {
        let response = client.get(address ).send()?.error_for_status()?;
        response.text()
    })?;
    Ok(response)
}
