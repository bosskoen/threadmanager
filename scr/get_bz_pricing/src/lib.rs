use data_base_manager::{
    ColumnDefinition, DataBaseError, SQLReadable, SQLformat,
};
use error_handeler::{ErrorOperation, RGB};
use library::{data_base_manager::{get_colum_value, PgRow, PostgresType, SyncConnection, ToSql}, error_handeler::Printer, *};
use parsing::BzData;
use std::{sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    }, thread, time::{Duration, Instant}
};
use type_dependecies::{Context, PricingError};
use parsing::DataBaseLogin;

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

    let mut context = Context::from(stopflag, status, settings_path)?;
    printer.send(ErrorOperation::ChangeLed(RGB::BLUE(), false, error_handeler::LedNumber::LED2), APP_NAME).map_err(|_| {
        Box::new(ErrorThreadDownError::new(APP_NAME, "error while turning on led"))
    })?;
    //TODO color

loop {
    let start = Instant::now();
    context.accumulated += start.duration_since(context.last_loop);
    context.last_loop = start;

    

    if context.stopflag.load(Ordering::Relaxed) {
        break;
    }

    if let Err(err) = context.update_timing(){
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
            return Err(Box::new(ErrorThreadDownError::new(APP_NAME, "error while updating timing, retrying next cycle")));
        }
    }

    // Check how much time passed since last update
    if context.accumulated >= Duration::from_secs(context.update_rate as u64) {
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
                        return Box::new(ErrorThreadDownError::new(APP_NAME, &format!("error while getting data from api, retrying next cycle\n{err}")))
                            as Box<dyn std::error::Error>;
                    };
                    Box::new(err) as Box<dyn std::error::Error>
                }
            })?;
            context.accumulated -= Duration::from_secs(context.update_rate as u64); // reset timer
    }

    let elapsed  = start.elapsed();

    let sleep_duration = Duration::from_secs(context.step_rate as u64)
        .checked_sub(elapsed)
        .unwrap_or(Duration::ZERO);

    if sleep_duration == Duration::ZERO{
        if let Err(_) = printer.send(
            ErrorOperation::PrintAndChangeLedError(
                APP_NAME.to_string(),
                "The loop took too long, skipping sleep".to_string(),
                RGB::ERROR(),
                WARNGING_ORANGE,
                error_handeler::LedNumber::LED2,
            ),
            APP_NAME,
        ){
            return Err(Box::new(ErrorThreadDownError::new(APP_NAME, "The loop took too long, skipping sleep")));
        }
    }
    thread::sleep(sleep_duration);
}
    printer.send(ErrorOperation::ChangeLed(RGB::BLACK(), false, error_handeler::LedNumber::LED2), APP_NAME).map_err(| _| {
        Box::new(ErrorThreadDownError::new(APP_NAME, &format!("error while turning of led")))
    })?;
    Ok(())
}

fn validate_data_base(
    login: DataBaseLogin,
    table_name: &str,
    lookup_table_name: &str,
) -> Result<(), PricingError> {
    use PostgresType as PT;

    let mut owned_conn = SyncConnection::new(
        &login.user_name,
        &login.password,
        &login.host,
        &login.database_name,
    )?; //TODO

    //CREATE TABLE [name](
    //ID INT NOT NULL,
    //timeStamp INT NOT NULL,
    //sellPrice REAL NOT NULL,
    //buyPrice REAL NOT NULL,
    //sellVolume INT NOT NULL,
    //sellMovingWeek INT NOT NULL,
    //buyVolume INT NOT NULL,
    //buyMovingWeek INT NOT NULL,
    //PRIMARY KEY (ID , TimeStamp)
    //);
    let collums = vec![
        define_column!("ID", PT::i16, true, true),
        define_column!("timeStamp", PT::i64, true, true),
        define_column!("sellPrice", PT::f64, true, false),
        define_column!("buyPrice", PT::f64, true, false),
        define_column!("sellVolume", PT::i32, true, false),
        define_column!("sellMovingWeek", PT::i64, true, false),
        define_column!("buyVolume", PT::i32, true, false),
        define_column!("buyMovingWeek", PT::i64, true, false),
    ];
    owned_conn.ensure_table_format(table_name, collums)?;

    //CREATE TABLE [name](
    //    HypixelID TEXT NOT NULL UNIQUE,
    //    ID INTEGER PRIMARY KEY AUTOINCREMENT,
    //    Name TEXT
    //);

   // check_and_create_lookup_table(owned_conn, lookup_table_name)?;

    Ok(())
}

/*fn check_and_create_lookup_table(conn: SyncConnection, table_name: &str) -> Result<(), PricingError> {
    // Step 1: Ensure the table exists
    create_table_if_not_exists(conn, table_name)?;

    // Step 2: Validate the table schema
    let columns = fetch_table_schema(conn, table_name)?;

    // Step 3: Handle missing columns (if any)
    if !validate_table_schema(&columns)? {
        ensure_column_exists(conn, table_name, "Name", "TEXT")?;
    }

    Ok(())
}

fn create_table_if_not_exists(conn: SyncConnection, table_name: &str) -> Result<(), PricingError> {
    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {}(
                HypixelID TEXT NOT NULL UNIQUE, 
                ID INTEGER PRIMARY KEY AUTOINCREMENT, 
                Name TEXT
            );",
            table_name
        ),
        [],
    )?;
    Ok(())
}

/// Fetches the table schema using `PRAGMA table_info`
/// returns true if column exists
fn fetch_table_schema(
    conn: &Connection,
    table_name: &str,
) -> Result<Vec<(String, String, isize)>, PricingError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({});", table_name))?;
    let column_info = stmt.query_map([], |row| {
        Ok((
            row.get::<usize, String>(1)?, // Column name
            row.get::<usize, String>(2)?, // Column type
            row.get::<usize, isize>(5)?,  // Is primary key? (1 if true, 0 if false)
        ))
    })?;
    column_info
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.into())
}

/// Validates the schema of the table
fn validate_table_schema(columns: &[(String, String, isize)]) -> Result<bool, PricingError> {
    let mut has_hypixel_id = false;
    let mut has_id = false;
    let mut has_name = false;
    let mut id_is_primary_key = false;
    let mut hypixel_id_is_unique = false;

    for (column_name, column_type, is_primary_key) in columns {
        match column_name.as_str() {
            "HypixelID" => {
                has_hypixel_id = true;
                if column_type != "TEXT" {
                    return Err(PricingError::SQLformatError(format!(
                        "Error: 'HypixelID' should be of type 'TEXT', found '{}'.",
                        column_type
                    )));
                }
                if *is_primary_key != 0 {
                    return Err(PricingError::SQLformatError(
                        "Error: 'HypixelID' should not be a primary key.".to_string(),
                    ));
                }
                hypixel_id_is_unique = true;
            }
            "ID" => {
                has_id = true;
                if column_type != "INTEGER" {
                    return Err(PricingError::SQLformatError(format!(
                        "Error: 'ID' should be of type 'INTEGER', found '{}'.",
                        column_type
                    )));
                }
                if *is_primary_key == 0 {
                    return Err(PricingError::SQLformatError(
                        "Error: 'ID' should be the primary key.".to_string(),
                    ));
                } else {
                    id_is_primary_key = true;
                }
            }
            "Name" => {
                has_name = true;
                if column_type != "TEXT" {
                    return Err(PricingError::SQLformatError(format!(
                        "Error: 'Name' should be of type 'TEXT', found '{}'.",
                        column_type
                    )));
                }
            }
            _ => {}
        }
    }

    // Ensure all required columns exist
    if !has_hypixel_id {
        return Err(PricingError::SQLformatError(
            "Error: Column 'HypixelID' is missing.".to_string(),
        ));
    }
    if !has_id {
        return Err(PricingError::SQLformatError(
            "Error: Column 'ID' is missing.".to_string(),
        ));
    }
    if !id_is_primary_key {
        return Err(PricingError::SQLformatError(
            "Error: 'ID' should be the primary key.".to_string(),
        ));
    }
    if !hypixel_id_is_unique {
        return Err(PricingError::SQLformatError(
            "Error: 'HypixelID' should be unique.".to_string(),
        ));
    }

    Ok(has_name)
}

/// Ensures a specific column exists in the table
fn ensure_column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    column_type: &str,
) -> Result<(), PricingError> {
    conn.execute(
        &format!(
            "ALTER TABLE {} ADD COLUMN {} {};",
            table_name, column_name, column_type
        ),
        [],
    )?;

    Ok(())
}*/

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

    printer.print(&data.text[..100], RGB::DEBUG()); //TODO

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
                return PricingError::ErrorThreadDown(
                    format!("error while parsing the json, retrying next cycle\n{err}\n"),
                );
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
    let hypixel_ids = json_data
        .products
        .values()
        .map(|value| Name {
            name: value.product_id.clone(),
        })
        .collect();

    if context.conn.try_write_database(
        hypixel_ids,
        &context.lookup_table_name,
        "HypixelID",
    )? > 0
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
                ToSql::i16(self.id ),
                ToSql::i64(self.time_stamp ),
                ToSql::f64(self.sell_price),
                ToSql::f64(self.buy_price),
                ToSql::i32(self.sell_volme ),
                ToSql::i64(self.sell_moving_week),
                ToSql::i32(self.buy_volme ),
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
                &format!("WHERE HypixelID = '{}'", value.product_id),
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
        .collect();

    context.conn.write_database(
        prices_data,
        &context.data_table_name,
        "ID, timeStamp, sellPrice, buyPrice, sellVolume, sellMovingWeek, buyVolume, buyMovingWeek",
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

    if json_data.last_updated == context.last_update{
        printer.print("sciped do to same data", RGB::DEBUG()); //TODO
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
