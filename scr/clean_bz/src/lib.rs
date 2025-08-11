use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use library::{
    DateTime, ErrorThreadDownError, Local, NaiveTime, Status, TimeDelta, TimeZone,
    data_base_manager::{
        ColumnDefinition, DataBaseError, PgPermission, SyncConnection,
        sqlx::{self, PgPool},
    },
    error_handeler::{ErrorOperation, Printer, RGB},
};

use crate::{
    parsing::CleaningProfiles,
    types::{CleanError, Context},
};

mod parsing;
mod types;

const APP_NAME: &str = "clean_bz";

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
                RGB::from_hex(0xac00e6),
                false,
                library::error_handeler::LedNumber::LED4,
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

        let mut now = Local::now();

        // Check how much time passed since last update
        if context.next_run <= now {
            // DO WORK
            let delted = clean_db(&mut context)?;
            context.update_status(delted)?;
        }

        now = Local::now();

        let margin = Duration::from_millis(2); // small safety net to avoid early wakeup

        let max_sleep = Duration::from_secs_f64(context.step_rate as f64);

        // Instead of subtracting a margin, we add it:
        let time_till_next_run = (context.next_run - now).to_std().unwrap_or(Duration::ZERO);

        let sleep_duration;
        if time_till_next_run <= max_sleep {
            sleep_duration = time_till_next_run + margin;
        } else {
            sleep_duration = max_sleep;
        }

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
                library::error_handeler::LedNumber::LED4,
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

pub fn confurm_db(
    table: &str,
    user: &str,
    owner: SyncConnection,
    printer: &Printer,
) -> Result<(), CleanError> {
    use library::data_base_manager::PostgresType as PT;

    let table_constaint: [ColumnDefinition; 2] = [
        ColumnDefinition::new("ID".to_string(), PT::i16, true, true, false, None),
        ColumnDefinition::new("time_Stamp".to_string(), PT::i64, true, true, false, None),
    ];

    if let Some(msg) =
        owner
            .ensure_table_format(table, &table_constaint)
            .map_err(|(extas, error)| {
                if let Some(msg) = extas {
                    printer.named_print(APP_NAME, &(msg + "."), RGB::TRACE());
                }
                CleanError::from(error)
            })?
    {
        printer.named_print(APP_NAME, &msg, RGB::TRACE());
    }
    owner.grant_permission(user, table, &[PgPermission::Delete, PgPermission::Select])?;
    Ok(())
}

fn clean_db(context: &mut Context) -> Result<u64, CleanError> {
    context.next_run = next_time_hour(context.update_time)?;

    let (pool, rt) = context.db_connection.get_inner();

    rt.block_on(delete_data(
        &pool,
        &context.cleaning_profiles,
        &context.table,
        &context.printer,
    ))
}

async fn delete_data(
    pool: &PgPool,
    profiles: &[CleaningProfiles],
    table: &str,
    printer: &Printer,
) -> Result<u64, CleanError> {
    let start = Instant::now();

    let now = Local::now().timestamp_millis();
    const DAY: i64 = 24 * 3600 * 1000;

    let mut num_deleted: Vec<u64> = Vec::new();

    printer
        .send(
            ErrorOperation::ChangeLed(RGB::BLUE(), false, library::error_handeler::LedNumber::LED4),
            APP_NAME,
        )
        .map_err(|_| CleanError::ErrorThreadDown("at start clean loop".to_string()))?;

    // 1 = interval
    // 2 = item to keep per interval
    //3 = day to keep data
    let string = format!(
        r#"
WITH ranked AS (
    SELECT
        id,
        time_stamp,
        ROW_NUMBER() OVER (
            PARTITION BY id, FLOOR(time_stamp / (({DAY} * $1 ) / $2))
            ORDER BY time_stamp ASC
        ) AS row_num
    FROM {table}
    WHERE time_stamp < {now} - $3 * {DAY}
)
DELETE FROM {table}
WHERE (id, time_stamp) IN (
    SELECT id, time_stamp
    FROM ranked
    WHERE row_num > 1
);"#
    );

    for profile in profiles {
        num_deleted.push(
            sqlx::query(&string)
                .bind(profile.sample_interval_days)
                .bind(profile.samples_to_keep_per_interval)
                .bind(profile.full_retention_days)
                .execute(pool)
                .await
                .map_err(|e| DataBaseError::from(e))?
                .rows_affected(),
        );
    }
    sqlx::query(&format!("VACUUM ANALYZE {table}"))
        .execute(pool)
        .await
        .map_err(|e| DataBaseError::from(e))?;

    let total_clean = num_deleted.iter().sum();

    #[cfg(debug_assertions)]
    printer.named_print(
        APP_NAME,
        &format!(
            "cleaned {} recoreds, in {}",
            total_clean,
            Instant::now().duration_since(start).as_secs()
        ),
        RGB::DEBUG(),
    );

    printer
        .send(
            ErrorOperation::ChangeLed(
                RGB::from_hex(0xac00e6),
                false,
                library::error_handeler::LedNumber::LED4,
            ),
            APP_NAME,
        )
        .map_err(|_| CleanError::ErrorThreadDown("at stop clean loop".to_string()))?;

    Ok(total_clean)
}

pub fn next_time_hour(hour: f32) -> Result<DateTime<Local>, CleanError> {
    let uur = hour.floor() as u32;
    let minute = ((hour - uur as f32) * 60.0).floor() as u32;

    let now = Local::now();

    // Construct today's target time
    let today_target_time = NaiveTime::from_hms_opt(uur, minute, 0).ok_or_else( ||CleanError::TimeError(format!("hour: \"{uur}\" or minute: \"{minute}\" are wrong. they need to be les than 24 and las thab 60 respectivly")))?;

    let today_date = now.date_naive();
    let today_target_naive = today_date.and_time(today_target_time);

    // Convert to DateTime<Local>
    let target_datetime = Local
        .from_local_datetime(&today_target_naive)
        .single()
        .ok_or(CleanError::TimeError(
            "coudn't creat the datatime".to_string(),
        ))?;

    // If that time is still ahead of us today, return it.
    // Otherwise, return it for tomorrow.
    Ok(if target_datetime > now {
        target_datetime
    } else {
        let tomorrow_date = today_date + TimeDelta::days(1);
        let tomorrow_naive = tomorrow_date.and_time(today_target_time);
        Local
            .from_local_datetime(&tomorrow_naive)
            .single()
            .ok_or(CleanError::TimeError(
                "coudn't creat the datatime".to_string(),
            ))?
    })
}
