pub mod error_handeler;
pub mod data_base_manager;
pub mod web_service_adapter;

use std::any::Any;

pub use chrono::{DateTime, Local, NaiveDateTime};

pub trait Status: Send + Sync {
    fn format(&self) -> String;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub fn format_duration(start: DateTime<Local>, end: DateTime<Local>) -> String {
    let duration = end - start;

    let total_days = duration.num_days();
    let months = total_days / 30; // Approximation, as months can vary in length
    let days = total_days % 30;
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    format!(
        "{} months, {} days, {:02}:{:02}:{:02}",
        months, days, hours, minutes, seconds
    )
}

#[macro_export]
macro_rules! impl_status {
    ($struct_name:ident, $format_body:expr) => {
        impl Status for $struct_name {
            fn format(&self) -> String {
                $format_body(self)
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}

#[macro_export]
macro_rules! throw_error {
    ($error_handle:expr, PrintAndChangeLed($message:expr, $color:expr)) => {
        if let Err(_) = $error_handle.send(ErrorOperation::PrintAndChangeLed($message.to_string(), $color)) {
            println!("Couldn't send error message: {}", $message);
            return 105;
        }
    };
    
    // For Print operation
    ($error_handle:expr, Print($message:expr)) => {
        if let Err(_) = $error_handle.send(ErrorOperation::Print($message.to_string())) {
            println!("Couldn't send error message: {}", $message);
            return 105;
        }
    };
    
    // For ChangLed operation
    ($error_handle:expr, ChangeLed($color:expr)) => {
        if let Err(_) = $error_handle.send(ErrorOperation::ChangLed($color)) {
            println!("Couldn't send error message to change LED color.");
            return 105;
        }
    };
    
    // For BlinkLed operation
    ($error_handle:expr, BlinkLed($color:expr, $duration:expr)) => {
        if let Err(_) = $error_handle.send(ErrorOperation::BlickLed($color, $duration)) {
            println!("Couldn't send error message to blink LED.");
            return 105;
        }
    };
    
    // For PrintAndBlinkLed operation
    ($error_handle:expr, PrintAndBlinkLed($message:expr, $color:expr, $duration:expr)) => {
        if let Err(_) = $error_handle.send(ErrorOperation::PrintAndBlinkLed($message.to_string(), $color, $duration)) {
            println!("Couldn't send error message: {}", $message);
            return 105;
        }
    };
}

#[macro_export]
macro_rules! throw_exit_error {
    ($error_handle:expr, PrintAndChangeLed($message:expr, $color:expr), $return_code:expr) => {
        if let Err(_) = $error_handle.send(ErrorOperation::PrintAndChangeLed($message.to_string(), $color)) {
            println!("Couldn't send error message: {}\nwith error code {}", $message, $return_code);
            return 105;
        }
        return $return_code;
    };
    
    // For Print operation
    ($error_handle:expr, Print($message:expr), $return_code:expr) => {
        if let Err(_) = $error_handle.send(ErrorOperation::Print($message.to_string())) {
            println!("Couldn't send error message: {}\nwith error code {}", $message,$return_code);
            return 105;
        }
        return $return_code;
    };
    
    // For ChangLed operation
    ($error_handle:expr, ChangeLed($color:expr), $return_code:expr) => {
        if let Err(_) = $error_handle.send(ErrorOperation::ChangLed($color)) {
            println!("Couldn't send error message to change LED color.\nwith error code {}",$return_code);
            return 105;
        }
        return $return_code;
    };
    
    // For BlinkLed operation
    ($error_handle:expr, BlinkLed($color:expr, $duration:expr), $return_code:expr) => {
        if let Err(_) = $error_handle.send(ErrorOperation::BlickLed($color, $duration)) {
            println!("Couldn't send error message to blink LED.\nwith error code {}",$return_code);
            return 105;
        }
        return $return_code;
    };
    
    // For PrintAndBlinkLed operation
    ($error_handle:expr, PrintAndBlinkLed($message:expr, $color:expr, $duration:expr), $return_code:expr) => {
        if let Err(_) = $error_handle.send(ErrorOperation::PrintAndBlinkLed($message.to_string(), $color, $duration)) {
            println!("Couldn't send error message: {}\nwith error code {}", $message, $return_code);
            return 105;
        }
        return $return_code;
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test (){

    }
}
