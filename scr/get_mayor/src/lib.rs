use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use library::{
    ErrorThreadDownError, Status,
    data_base_manager::{
        ColumnDefinition, DataBaseError, PgPermission, PostgresType, SyncConnection,
        get_colum_value, sqlx,
    },
    error_handeler::{ErrorOperation, Printer, RGB},
    web_service_adapter::get_data_plus_size,
};

use crate::{
    parsing::{DataBaseLogin, MayorData},
    types::{Context, MayorError, Mode},
};

const APP_NAME: &str = "mayor_tracker";

mod parsing;
mod types;

//MAIN FUNC
#[unsafe(no_mangle)]
pub fn start(
    printer: Printer,
    stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::from(stopflag, status, settings_path, &printer)?;

     printer
        .send(
            library::error_handeler::ErrorOperation::ChangeLed(
                RGB::from_hex(0x19eb5d),
                false,
                library::error_handeler::LedNumber::LED3,
            ),
            APP_NAME,
        )
        .map_err(|_| {
            Box::new(ErrorThreadDownError::new(
                APP_NAME,
                "error while turning on led",
            ))
        })?;
    //TODO color

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
                    RGB::ERROR(),
                ),
                APP_NAME,
            ) {
                return Err(Box::new(ErrorThreadDownError::new(
                    APP_NAME,
                    "error while updating timing, retrying next cycle",
                )));
            }
        }

        let update_interval;
        match context.mode {
            Mode::CatchUp | Mode::Polling => {
                update_interval = Duration::from_secs_f64(context.update_rate as f64);
                if context.accumulated >= update_interval {
                    // do work
                    if let Err(e) = get_mayor(&mut context) {
                        printer.named_print(
                            APP_NAME,
                            &format!("got error: {e}\n retrying next cycle"),
                            RGB::ALERT(),
                        );
                    }

                    // reset timer
                    context.accumulated -= update_interval;
                }
            }
            Mode::Waiting => {
                update_interval =
                    Duration::from_secs_f64((context.mayor_reletc - context.window) as f64);
                if context.accumulated >= update_interval {
                    // do work

                    context.mode = Mode::Polling;
                    context
                        .update_status(None, 0)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    // reset timer
                    context.accumulated -= update_interval;
                }
            }
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
                    RGB::ERROR(),
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

    printer
        .send(
            library::error_handeler::ErrorOperation::ChangeLed(
                RGB::BLACK(),
                false,
                library::error_handeler::LedNumber::LED2,
            ),
            APP_NAME,
        )
        .map_err(|_| {
            Box::new(ErrorThreadDownError::new(
                APP_NAME,
                &format!("error while turning of led"),
            ))
        })?;

    Ok(())
}

fn get_mayor(context: &mut Context) -> Result<(), MayorError> {
    /*get mayor and time stapm
    compare mayor to curent mayor
    if not equel
        add to data base
        update curent mayor
        set mode to wait
    update status
    */

    let data = get_data_plus_size(&context.url, 3, Duration::from_secs(3))?;

    let data_used = data.received_bytes + data.sent_bytes;
    let mayor_data = MayorData::get(&data.text)?;

    let mut name = None;
    if context.mayor_name != mayor_data.name {
        context.database_user.write_database(
            &[mayor_data.clone()],
            &context.table_name,
            "time, mayor, year",
        )?;
        context.mayor_name = mayor_data.name.clone();
        name = Some((unix_to_sys(mayor_data.time).into(), mayor_data.name));
        context.mode = Mode::Waiting
    }
    context.update_status(name, data_used)?;

    Ok(())
}

fn unix_to_sys(unix: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(unix as u64)
}

pub fn get_current_mayer(
    user: &SyncConnection,
    table_name: &String,
) -> Result<Option<MayorData>, MayorError> {
    let (pool, rt) = user.get_inner();

    let opt_row = rt.block_on(
        sqlx::query(&format!(
            "SELECT mayor, time, year FROM {} ORDER BY time DESC LIMIT 1",
            table_name
        ))
        .fetch_one(&pool),
    );

    let option = match opt_row {
        Ok(row) => Some(MayorData{
            name: get_colum_value::<String>(&row, "mayor")?,
            time: get_colum_value::<i64>(&row, "time")?,
            year: get_colum_value::<i32>(&row, "year")?
        }
        ),
        Err(e) => match e {
            sqlx::Error::RowNotFound => None,
            _ => Err(MayorError::from(DataBaseError::from(e)))?,
        },
    };

    Ok(option)
}

pub fn enshure_database(
    database_owner: DataBaseLogin,
    table_name: &str,
    user: &str,
    printer: &Printer,
) -> Result<(), MayorError> {
    let owner = SyncConnection::new(
        &database_owner.user_name,
        &database_owner.password,
        &database_owner.host,
        &database_owner.database_name,
    )?;

    let columns = vec![
        ColumnDefinition::new(
            "mayor".to_string(),
            PostgresType::String,
            true,
            false,
            false,
            None,
        ),
        ColumnDefinition::new(
            "time".to_string(),
            PostgresType::i64,
            true,
            true,
            true,
            None,
        ),
        ColumnDefinition::new(
            "year".to_string(),
            PostgresType::i32,
            true,
            false,
            true,
            None,
        ),
    ];

    if let Some(msg) =
        owner
            .ensure_table_format(table_name, &columns)
            .map_err(|(extas, error)| {
                if let Some(msg) = extas {
                    printer.named_print(APP_NAME, &(msg + "."), RGB::TRACE());
                }
                MayorError::from(error)
            })?
    {
        printer.named_print(APP_NAME, &msg, RGB::TRACE());
    }

    owner.grant_permission(
        user,
        table_name,
        &[PgPermission::Insert, PgPermission::Select],
    )?;

    Ok(())
}
