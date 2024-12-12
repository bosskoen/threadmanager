use std::fmt;


pub enum Error {
    
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl From<reqwest::Error> for Error{
    fn from(value: reqwest::Error) -> Self {
        todo!()
    }
}
impl From<reqwest::Url::ParseError> for Error {
    fn from(value: reqwest::Url::ParseError) -> Self {
        todo!()
    }
}