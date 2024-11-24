pub mod error_handeler;

use std::any::Any;

pub trait Status: Send + Sync {
    fn format(&self) -> String;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(test)]
mod tests {
}
