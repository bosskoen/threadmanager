use std::{io::Read, time::Duration};

use reqwest::blocking;
use retry::{delay::Fixed, retry};
use url::Url;

pub use custom_types::*;

mod custom_types;

pub fn get_data(
    address: &str,
    retrys: usize,
    timeout: Duration,
) -> Result<String, WebServiceError> {
    Url::parse(address)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let response = retry(Fixed::from_millis(100).take(retrys), || {
        let response = client.get(address).send()?.error_for_status()?;
        response.text()
    })?;
    Ok(response)
}

pub struct DataSized {
    pub text: String,
    pub sent_bytes: usize,
    pub received_bytes: usize,
}

pub fn get_data_plus_size(
    address: &str,
    retrys: usize,
    timeout: Duration,
) -> Result<DataSized, WebServiceError> {
    Url::parse(address)?; // Parsing the URL to validate it
    let client = blocking::Client::builder().timeout(timeout).build()?;

    let request = client.get(address).build()?;

    let mut sent_bytes = 0;
    let mut received_bytes = 0;

    if let Some(body) = request.body() {
        sent_bytes += body.as_bytes().unwrap_or(&[]).len();
    }

    let mut response = retry(Fixed::from_millis(100).take(retrys), || {
        client.get(address).send()?.error_for_status()
    })?;

    let mut body = Vec::new();
    response.read_to_end(&mut body)?;
    received_bytes += body.len();

    let text = String::from_utf8_lossy(&body).into_owned();

    Ok(DataSized {
        text,
        sent_bytes,
        received_bytes,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() -> Result<(), WebServiceError> {
        get_data(
            "https://api.hypixel.net/v2/skyblock/bazaar",
            2,
            Duration::from_secs(5),
        )?;
        let data = get_data_plus_size(
            "https://api.hypixel.net/v2/skyblock/bazaar",
            3,
            Duration::from_secs(5),
        )?;

        println!("sent {}\nreseved {}", data.sent_bytes, data.received_bytes);
        Ok(())
    }
}
