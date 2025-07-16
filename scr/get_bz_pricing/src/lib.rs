use data_base_manager::{ColumnDefinition, DataBaseError, SQLReadable, SQLformat};
use error_handeler::{ErrorOperation, RGB};
use library::{
    data_base_manager::{
        get_colum_value, PgPermission, PgRow, PgSequencePermission, SyncConnection, ToSql,
    },
    error_handeler::Printer,
    *,
};
use parsing::BzData;
use parsing::DataBaseLogin;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant}, vec,
};
use type_dependecies::{Context, PricingError};

mod parsing;
mod type_dependecies;

const APP_NAME: &str = "bz_tracker";
const WARNGING_ORANGE: RGB = RGB::from_hex(0xff7d00);

#[no_mangle]
pub fn start(
    mut printer: Printer,
    stopflag: Arc<AtomicBool>,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::from(stopflag, status, settings_path, &printer)?;
    printer
        .send(
            ErrorOperation::ChangeLed(RGB::BLUE(), false, error_handeler::LedNumber::LED2),
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

        if let Err(err) = context.update_timing() {
            if let Err(_) = printer.send(
                ErrorOperation::PrintAndChangeLedError(
                    APP_NAME.to_string(),
                    format!("error while updating timing, retrying next cycle\n{err}"),
                    RGB::ERROR(),
                    WARNGING_ORANGE,
                    error_handeler::LedNumber::LED2,
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
            get_bz_data(&mut printer, &mut context).map_err(|err| match err {
                PricingError::ErrorThreadDown(messige) => {
                    Box::new(ErrorThreadDownError::new(APP_NAME, &messige))
                }
                _ => {
                    if let Err(_) = printer.send(
                        ErrorOperation::PrintAndChangeLedError(
                            APP_NAME.to_string(),
                            format!(
                                "error while getting data from api, retrying next cycle\n{err}"
                            ),
                            RGB::ERROR(),
                            WARNGING_ORANGE,
                            error_handeler::LedNumber::LED2,
                        ),
                        APP_NAME,
                    ) {
                        return Box::new(ErrorThreadDownError::new(
                            APP_NAME,
                            &format!(
                                "error while getting data from api, retrying next cycle\n{err}"
                            ),
                        )) as Box<dyn std::error::Error>;
                    };
                    Box::new(err) as Box<dyn std::error::Error>
                }
            })?;
            context.accumulated -= update_interval;
            // reset timer
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
                ErrorOperation::PrintAndChangeLedError(
                    APP_NAME.to_string(),
                    "The loop took too long, skipping sleep".to_string(),
                    RGB::ERROR(),
                    WARNGING_ORANGE,
                    error_handeler::LedNumber::LED2,
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
            ErrorOperation::ChangeLed(RGB::BLACK(), false, error_handeler::LedNumber::LED2),
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

fn validate_data_base(
    login: DataBaseLogin,
    table_name: &str,
    lookup_table_name: &str,
    normal_user: &str,
    printer: &Printer,
) -> Result<(), PricingError> {
    use library::data_base_manager::PostgresType as PT;

    let owned_conn = SyncConnection::new(
        &login.user_name,
        &login.password,
        &login.host,
        &login.database_name,
    )?;

    //CREATE TABLE [name](
    //ID INT NOT NULL,
    //timeStamp INT NOT NULL,
    //sell_Price REAL NOT NULL,
    //buyPrice REAL NOT NULL,
    //sellVolume INT NOT NULL,
    //sellMovingWeek INT NOT NULL,
    //buyVolume INT NOT NULL,
    //buyMovingWeek INT NOT NULL,
    //PRIMARY KEY (ID , TimeStamp)
    //);
    let collums = vec![
        ColumnDefinition::new("ID".to_string(), PT::i16, true, true, false, None),
        ColumnDefinition::new("time_Stamp".to_string(), PT::i64, true, true, false, None),
        ColumnDefinition::new("sell_Price".to_string(), PT::f64, true, false, false, None),
        ColumnDefinition::new("buy_Price".to_string(), PT::f64, true, false, false, None),
        ColumnDefinition::new("sell_Volume".to_string(), PT::i32, true, false, false, None),
        ColumnDefinition::new(
            "sell_Moving_Week".to_string(),
            PT::i64,
            true,
            false,
            false,
            None,
        ),
        ColumnDefinition::new("buy_Volume".to_string(), PT::i32, true, false, false, None),
        ColumnDefinition::new(
            "buy_Moving_Week".to_string(),
            PT::i64,
            true,
            false,
            false,
            None,
        ),
    ];
    if let Some(msg) = owned_conn
        .ensure_table_format(table_name, &collums)
        .map_err(|(extas, error)| {
            if let Some(msg) = extas {
                printer.named_print(APP_NAME, &(msg + "."), RGB::TRACE());
            }
            PricingError::from(error)
        })?
    {
        printer.named_print(APP_NAME, &msg, RGB::TRACE());
    }


    //CREATE TABLE [name]( //sqlite
    //    Hypixel_ID TEXT NOT NULL UNIQUE,
    //    ID INTEGER PRIMARY KEY AUTOINCREMENT,
    //    Name TEXT
    //);

    let collums = vec![
        ColumnDefinition::new(
            "Hypixel_ID".to_string(),
            PT::String,
            true,
            false,
            true,
            None,
        ),
        ColumnDefinition::new("ID".to_string(), PT::i16Auto, true, true, true, None),
        ColumnDefinition::new("Name".to_string(), PT::String, false, false, false, None),
    ];

    if let Some(msg) = owned_conn
        .ensure_table_format(lookup_table_name, &collums)
        .map_err(|(extas, error)| {
            if let Some(msg) = extas {
                printer.named_print(APP_NAME, &msg, RGB::TRACE());
            }
            PricingError::from(error)
        })?
    {
        printer.named_print(APP_NAME, &msg, RGB::TRACE());
    }


    owned_conn.grant_permission(normal_user, table_name, &[PgPermission::Insert])?;
    owned_conn.grant_permission(
        normal_user,
        lookup_table_name,
        &[PgPermission::Insert, PgPermission::Select],
    )?;
    owned_conn.grant_sequence_premission(
        normal_user,
        &(lookup_table_name.to_string() + "_id_seq"),
        &[PgSequencePermission::All],
    )?;

    Ok(())
}

fn get_data_from_api(
    printer: &mut Printer,
    context: &Context,
) -> Result<(BzData, (usize, usize)), PricingError> {
    let data = web_service_adapter::get_data_plus_size(&context.url, 3, Duration::from_secs(3))
        .map_err(|err| {
            if let Err(_) = printer.send(
                ErrorOperation::PrintAndChangeLedError(
                    APP_NAME.to_string(),
                    format!("error while fetching data from api, retrying next cycle\n{err}"),
                    RGB::WARNING(),
                    WARNGING_ORANGE,
                    error_handeler::LedNumber::LED2,
                ),
                APP_NAME,
            ) {
                return PricingError::ErrorThreadDown(format!(
                    "error while fetching data from api, retrying next cycle\n{err}"
                ));
            }
            PricingError::NonFatal
        });

    let data = data?;

    let json_data = BzData::from_data(data.text).map_err(|err| match err {
        PricingError::JSONReadError => {
            if let Err(_) = printer.send(
                ErrorOperation::PrintAndChangeLedError(
                    APP_NAME.to_string(),
                    format!("error while parsing the json, retrying next cycle\n{err}\n"),
                    RGB::ERROR(),
                    WARNGING_ORANGE,
                    error_handeler::LedNumber::LED2,
                ),
                APP_NAME,
            ) {
                return PricingError::ErrorThreadDown(format!(
                    "error while parsing the json, retrying next cycle\n{err}\n"
                ));
            }
            PricingError::NonFatal
        }
        PricingError::JSONFormatError(message) => {
            if let Err(_) = printer.send(
                ErrorOperation::PrintAndChangeLedError(
                    APP_NAME.to_string(),
                    format!("error while parsing the json, retrying next cycle\n{message}"),
                    RGB::ERROR(),
                    WARNGING_ORANGE,
                    error_handeler::LedNumber::LED2,
                ),
                APP_NAME,
            ) {
                return PricingError::ErrorThreadDown(format!(
                    "error while parsing the json, retrying next cycle\n{message}"
                ));
            }
            PricingError::NonFatal
        }
        _ => err,
    })?;

    if !json_data.success {
        if let Err(_) = printer.send(
            ErrorOperation::PrintAndChangeLedError(
                APP_NAME.to_string(),
                "JSON had an unexpected error, retrying next cycle".to_string(),
                RGB::ERROR(),
                WARNGING_ORANGE,
                error_handeler::LedNumber::LED2,
            ),
            APP_NAME,
        ) {
            return Err(PricingError::ErrorThreadDown(
                "JSON had an unexpected error, retrying next cycle".to_string(),
            ));
        }
        return Err(PricingError::NonFatal);
    }

    Ok((json_data, (data.sent_bytes, data.received_bytes)))
}

fn update_index_database(
    json_data: &BzData,
    context: &mut Context,
    printer: &mut Printer,
) -> Result<(), PricingError> {
    struct Name {
        name: String,
    }
    impl SQLformat<'_> for Name {
        fn sqlformat(&self) -> Vec<ToSql> {
            vec![ToSql::Text(&self.name)]
        }
    }

    let hypixel_ids: Vec<Name> = json_data
        .products
        .values()
        .map(|value| Name {
            name: value.product_id.clone(),
        })
        .collect();

    let (pool, rt) = context.conn.get_inner();

    let existing_keys: HashSet<String> = rt
        .block_on(
            library::data_base_manager::sqlx::query_scalar(&format!(
                r#"SELECT {} FROM {} WHERE {} = ANY($1)"#,
                "Hypixel_ID", &context.lookup_table_name, "Hypixel_ID"
            ))
            .bind(
                &hypixel_ids
                    .iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<&str>>(),
            )
            .fetch_all(&pool),
        )
        .map_err(|e| DataBaseError::from(e))?
        .into_iter()
        .collect();

    let hypixel_ids: Vec<Name> = hypixel_ids.into_iter().filter(| n| !existing_keys.contains(&n.name)).collect();


    if context
        .conn
        .try_write_database(hypixel_ids, &context.lookup_table_name, "Hypixel_ID")?
        > 0
    {
        if let Err(_) = printer.send(
            ErrorOperation::NonErrorPrintAndChangeLed(
                APP_NAME.to_string(),
                "new item added to the database, requires manual naming.".to_string(),
                RGB::INFO(),
                RGB::MAGENTA(),
                error_handeler::LedNumber::LED2,
            ),
            APP_NAME,
        ) {
            return Err(PricingError::ErrorThreadDown(
                "new item added to the database, requires manual naming.".to_string(),
            ));
        }
    };

    Ok(())
}

fn write_database(
    context: &mut Context,
    json_data: BzData,
    printer: &mut Printer,
) -> Result<(bool, String), PricingError> {
    struct BazaData {
        id: i16,
        time_stamp: i64,
        sell_price: f64,
        buy_price: f64,
        sell_volme: i32,
        sell_moving_week: i64,
        buy_volme: i32,
        buy_moving_week: i64,
    }
    impl SQLformat<'_> for BazaData {
        fn sqlformat(&self) -> Vec<ToSql> {
            vec![
                ToSql::i16(self.id),
                ToSql::i64(self.time_stamp),
                ToSql::f64(self.sell_price),
                ToSql::f64(self.buy_price),
                ToSql::i32(self.sell_volme),
                ToSql::i64(self.sell_moving_week),
                ToSql::i32(self.buy_volme),
                ToSql::i64(self.buy_moving_week),
            ]
        }
    }
    struct Id {
        id: i16,
    }
    impl SQLReadable for Id {
        fn from_row(row: &PgRow) -> Result<Self, DataBaseError> {
            let id = get_colum_value::<i16>(&row, "ID")?;
            Ok(Id { id })
        }
    }

    let mut error_send_failed = false;
    let mut error_message: Vec<String> = Vec::new();
    let time = json_data.last_updated as i64;
    let prices_data = json_data
        .products
        .into_values()
        .filter_map(|value| {
            match context.conn.read_database::<Id>(
                &context.lookup_table_name,
                "ID",
                &format!("WHERE Hypixel_ID = '{}'", value.product_id),
            ) {
                Ok(id) => Some(BazaData {
                    id: id[0].id,
                    time_stamp: time,
                    sell_price: value.quick_status.sellPrice,
                    sell_volme: value.quick_status.sellVolume,
                    buy_moving_week: value.quick_status.buyMovingWeek,
                    buy_volme: value.quick_status.buyVolume,
                    sell_moving_week: value.quick_status.sellMovingWeek,
                    buy_price: value.quick_status.buyPrice,
                }),
                Err(e) => {
                    if let Err(_) = printer.send(
                        ErrorOperation::PrintAndChangeLedError(
                            APP_NAME.to_string(),
                            format!(
                                "couldn't read product id \"{}\" from database\nError: {}",
                                value.product_id, e
                            ),
                            RGB::ERROR(),
                            RGB::ERROR(),
                            error_handeler::LedNumber::LED2,
                        ),
                        APP_NAME,
                    ) {
                        error_send_failed = true;
                        error_message.push(format!(
                            "couldn't read product id \"{}\" from database\nError: {}",
                            value.product_id, e
                        ));
                    }
                    None
                }
            }
        })
        .collect::<Vec<BazaData>>();

    context.conn.write_database(
        &prices_data,
        &context.data_table_name,
        "ID, time_Stamp, sell_Price, buy_Price, sell_Volume, sell_Moving_Week, buy_Volume, buy_Moving_Week",
    )?;

    Ok((error_send_failed, error_message.join(";\n")))
}

fn get_bz_data(printer: &mut Printer, context: &mut Context) -> Result<(), PricingError> {
    // get data
    let json_data = get_data_from_api(printer, context);
    if let Err(PricingError::NonFatal) = json_data {
        return Ok(());
    }
    let (json_data, (data_out, data_in)) = json_data?;

    if json_data.last_updated == context.last_update {
        printer.print("skiped do to same data", RGB::DEBUG()); //TODO
        return Ok(()); // no new data
    }

    context.last_update = json_data.last_updated;

    // write new items
    update_index_database(&json_data, context, printer)?;

    // write to database
    let num_items = json_data.products.len();
    let (error_send_failed, error_message) = write_database(context, json_data, printer)?;

    context.update_status(num_items, data_in + data_out)?;

    if error_send_failed {
        return Err(PricingError::ErrorThreadDown(error_message));
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
