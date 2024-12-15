use std::fmt;
use url::ParseError;
use reqwest::Error as ReqwestError;
use retry::Error as RetryError;

// Define the WebServiceError enum
pub enum WebServiceError {
    UrlParseError(ParseError),
    HttpClientError(ReqwestError),
    RetryError(String),
    StdError(std::io::Error)
}

impl fmt::Display for WebServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebServiceError::UrlParseError(err) => write!(f, "URL parse error: {}", err),
            WebServiceError::HttpClientError(err) => write!(f, "HTTP client error: {}", err),
            WebServiceError::RetryError(err) => write!(f, "Retry error: {}", err),
            WebServiceError::StdError(err) => write!(f, "Standard Error: {}", err),
        }
    }
}

impl fmt::Debug for WebServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// Conversion from reqwest::Error to WebServiceError
impl From<ReqwestError> for WebServiceError {
    fn from(value: ReqwestError) -> Self {
        WebServiceError::HttpClientError(value)
    }
}

// Conversion from url::ParseError to WebServiceError
impl From<ParseError> for WebServiceError {
    fn from(value: ParseError) -> Self {
        WebServiceError::UrlParseError(value)
    }
}

// Conversion from retry::Error to WebServiceError
impl<E: fmt::Debug> From<RetryError<E>> for WebServiceError {
    fn from(value: RetryError<E>) -> Self {
        WebServiceError::RetryError(format!("{:?}", value))
    }
}

impl From<std::io::Error> for WebServiceError {
    fn from(value: std::io::Error) -> Self {
        WebServiceError::StdError(value)
    }
}