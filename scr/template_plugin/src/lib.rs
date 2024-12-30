use std::{sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};

use error_handeler::{ErrorOperation, RGB};
use library::*;
use min_dependencies::*;

mod min_dependencies;

const APP_NAME: &str = "template_plugin";

fn test_error_thread(error_handel: &Sender<ErrorOperation>) -> Result<(), PluginError> {
    if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), "test print".to_string(), RGB::BLUE())) {
        return Err(PluginError::ErrorThreadDown("test print".to_string()));
    }
    Ok(())
}

#[no_mangle]
pub fn start(error_handel: Sender<ErrorOperation>, stopflag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::from(stopflag, status, settings_path)?;

    test_error_thread(&error_handel).map_err(|err| match err {
        PluginError::ErrorThreadDown(message) => Box::new(ErrorThreadDownError::new(APP_NAME, &message)) as Box<dyn std::error::Error>,
        _ => Box::new(err) as Box<dyn std::error::Error>,
    })?;

    loop {
        let start_of_loop = SystemTime::now();
        context.update_timing()?;
        if context.stopflag.load(Ordering::Relaxed) {
            break;
        }
        if context.time_passed >= context.update_rate {
            context.time_passed = 0;
            if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), "test plugin action 'print'".to_string(), RGB::from_hex(0x951fcc))) {
                return Err(Box::new(ErrorThreadDownError::new(APP_NAME, "test plugin action 'print'")));
            }
            context.update_status()?
        } else {
            context.time_passed += context.step_rate;
        }

        let endloop = match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {
                if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), format!("error while getting elapsed time: {}", error), RGB::ERROR())) {
                    return Err(Box::new(ErrorThreadDownError::new(APP_NAME, &format!("error while getting elapsed time: {}", error))));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), "loop took too long".to_string(), RGB::WARNING())) {
                return Err(Box::new(ErrorThreadDownError::new(APP_NAME, "loop took too long")));
            }
            context.time_passed += (endloop.saturating_sub(Duration::from_secs(context.step_rate as u64))).as_secs() as usize;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
}
