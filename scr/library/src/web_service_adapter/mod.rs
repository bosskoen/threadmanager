use std::{io::Read, time::Duration};

use reqwest::blocking;
use url::Url;
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

pub struct DataSized {
    pub text: String,
    pub sent_bytes: usize,
    pub received_bytes: usize,
}

pub fn get_data_plus_size(address: &str, retrys: usize, timeout: Duration) -> Result<DataSized, WebServiceError> {
    Url::parse(address)?;  // Parsing the URL to validate it
    let client = blocking::Client::builder()
        .timeout(timeout)
        .build()?;

    let request = client.get(address).build()?;
    
    let mut sent_bytes = 0;
    let mut received_bytes = 0;

    if let Some(body) = request.body() {
        sent_bytes += body.as_bytes().unwrap_or(&[]).len();
    }

    let mut response = retry(Fixed::from_millis(100).take(retrys), || {
        client.get(address).send()?.error_for_status()
    })?;

    if let Some(content_length) = response.content_length() {
        received_bytes += content_length as usize;
    } else {
        let mut body:Vec<u8> = Vec::new();
        received_bytes += response.read_to_end(&mut body)?;
    }

    Ok(DataSized {
        text: response.text()?,
        sent_bytes,
        received_bytes,
    })
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test() -> Result<() ,WebServiceError>{
        get_data("https://api.hypixel.net/v2/skyblock/bazaar", 2, Duration::from_secs(5))?;
        let data = get_data_plus_size("https://api.hypixel.net/v2/skyblock/bazaar", 3, Duration::from_secs(5))?;

        println!("sent {}\nreseved {}", data.sent_bytes, data.received_bytes);
        Ok(())
    }
}