use std::{sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, thread, time::{Duration, Instant}};

use error_handeler::{ErrorOperation, RGB};
use library::{error_handeler::Printer, *};
use min_dependencies::*;

mod min_dependencies;

const APP_NAME: &str = "template_plugin";

fn test_error_thread(printer: &mut Printer) -> Result<(), PluginError> {
    if let Err(_) = printer.send(ErrorOperation::NonErrorPrint(APP_NAME.to_string(), "test print".to_string(), RGB::BLUE()), APP_NAME) {
        return Err(PluginError::ErrorThreadDown("test print".to_string()));
    }
    Ok(())
}

#[no_mangle]
pub fn start(mut printer: Printer, stopflag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::from(stopflag, status, settings_path)?;

    test_error_thread(&mut printer).map_err(|err| match err {
        PluginError::ErrorThreadDown(message) => Box::new(ErrorThreadDownError::new(APP_NAME, &message)) as Box<dyn std::error::Error>,
        _ => Box::new(err) as Box<dyn std::error::Error>,
    })?;

    loop {
        let start = Instant::now();
        context.accumulated += start.duration_since(context.last_loop);
        context.last_loop = start;

        if context.stopflag.load(Ordering::Relaxed) {
            break;
        }

        // reget timing/settings data and the error hendeling
        if let Err(err) = context.update_timing() {
            if let Err(_) = printer.send(
                ErrorOperation::PrintError(
                    APP_NAME.to_string(),
                    format!("error while updating timing, retrying next cycle\n{err}"),
                    RGB::ERROR()
                ),
                APP_NAME,
            ) {
                return Err(Box::new(ErrorThreadDownError::new(
                    APP_NAME,
                    "error while updating timing, retrying next cycle",
                )));
            }
        }

        let update_interval = Duration::from_secs_f64(context.update_rate as f64);

        // Check how much time passed since last update
        if context.accumulated >= update_interval {
            // do work
            printer.named_print(APP_NAME, "test print", RGB::BLUE());
            context.update_status()?;

            // reset timer
            context.accumulated -= update_interval;
            
        }

        let elapsed = start.elapsed();

       let margin = Duration::from_millis(2); // small safety net to avoid early wakeup

    let max_sleep = Duration::from_secs_f64(context.step_rate as f64)
    .checked_sub(elapsed)
    .unwrap_or(Duration::ZERO);

    // Instead of subtracting a margin, we add it:
    let biased_time_until_update = update_interval
    .checked_sub(context.accumulated)
    .map(|d| d.saturating_add(margin)) // safe add without overflow
    .unwrap_or(Duration::ZERO);

    let sleep_duration = std::cmp::min(max_sleep, biased_time_until_update);

        if sleep_duration == Duration::ZERO {
            if let Err(_) = printer.send(
                ErrorOperation::PrintError(
                    APP_NAME.to_string(),
                    "The loop took too long, skipping sleep".to_string(),
                    RGB::ERROR()
                ),
                APP_NAME,
            ) {
                return Err(Box::new(ErrorThreadDownError::new(
                    APP_NAME,
                    "The loop took too long, skipping sleep",
                )));
            }
        }
        thread::sleep(sleep_duration);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
}
