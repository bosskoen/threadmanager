pub mod error_handeler;
pub mod data_base_manager;

use std::any::Any;

use chrono::{DateTime, Local};

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
                $format_body
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test (){

    }
}
