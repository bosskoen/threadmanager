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

/// (text,(sent_bytes, received_bytes))
pub fn get_data_puls_size(address: &str, retrys: usize, timeout: Duration) -> Result<(String,(usize,usize)), WebServiceError> {
    Url::parse(address)?;  // Parsing the URL to validate it
    let client = blocking::Client::builder()
        .timeout(timeout)
        .build()?;

    let request = client.post(address).build()?;
    
    let mut sent_bytes = 0;
    let mut received_bytes = 0;

    if let Some(body) = request.body() {
        sent_bytes += body.as_bytes().unwrap_or(&[]).len();    //TODO body is empy so is allways 0 but the hole function in inacuret
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

    Ok((response.text()?,(sent_bytes,received_bytes)))
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn test() -> Result<() ,WebServiceError>{
        get_data("https://api.hypixel.net/v2/skyblock/bazaar", 2, Duration::from_secs(5))?;
        let (_,(sent, reseved)): (String,(usize,usize)) = get_data_puls_size("https://api.hypixel.net/v2/skyblock/bazaar", 3, Duration::from_secs(5))?;

        println!("sent {}\nreseved {}", sent, reseved);
        Ok(())
    }
}