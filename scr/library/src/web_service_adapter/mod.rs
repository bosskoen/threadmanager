use reqwest::Url;
use retry::{retry, delay::Fixed};

pub use custom_types::*;

mod custom_types;

pub fn get_data(adress: &str) -> Result<String, Error>{
    Url::parse(adress)?;
    let response: Result<String, Error> = retry( Fixed::from_millis(10) ,|| {
        let response =reqwest::blocking::get(adress)?.error_for_status()?;
        response.text()
    });
    response
}