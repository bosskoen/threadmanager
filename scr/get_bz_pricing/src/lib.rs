use std::{sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};
use library::*;
use data_base_manager::{rusqlite::{self, ToSql}, ColumnDefinition, Connection, DataBaseError, SQLReadable, SQLformat};
use error_handeler::ErrorOperation;
use parsing::BzData;
use type_dependecies::{Context, PricingError};

mod parsing;
mod type_dependecies;

const APP_NAME: &str = "get_bz_pricing";


#[no_mangle]
pub fn start(error_handel: Sender<ErrorOperation>, stopflag: Arc<AtomicBool>, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>>{

    let mut context = Context::from(stopflag, status, settings_path)?;

    loop {
        let start_of_loop = SystemTime::now();
        context.update_timing()?; 
        if context.stopflag.load(Ordering::Relaxed){
            break;
        }
        if context.time_passed >= context.update_rate{
            context.time_passed = 0;
            get_bz_data(&error_handel, &mut context).map_err(|err|match err {
                PricingError::ErrorThreadDown(messige) => Box::new(ErrorThreadDownError::new(APP_NAME, &messige)),
                _ => Box::new(err) as Box<dyn std::error::Error>
            })?;
        }else{
            context.time_passed += context.step_rate;
        }

        let endloop =match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {
                if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),format!("error while getting elepsted time: {error}"))){
                    return Err(Box::new(ErrorThreadDownError::new(APP_NAME,&format!("error while getting elepsted time: {error}"))));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"loop took to long in price logger".to_string())){
                return Err(Box::new(ErrorThreadDownError::new(APP_NAME, "loop took to long in price logger")));
            }
            context.time_passed += (endloop.saturating_sub(Duration::from_secs(context.step_rate as u64))).as_secs() as usize;
        }
    }
    Ok(())
}

fn validate_data_base(conn: &mut Connection,table_name: &str,lookup_table_name: &str ) -> Result<(), PricingError>{
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
    let collums = vec![define_column!("ID","INT", true, true),
    define_column!("timeStamp","INT", true, true),
    define_column!("sellPrice","REAL", true, false),
    define_column!("buyPrice","REAL", true, false),
    define_column!("sellVolume","INT", true, false),
    define_column!("sellMovingWeek","INT", true, false),
    define_column!("buyVolume","INT", true, false),
    define_column!("buyMovingWeek","INT", true, false)  ];
    data_base_manager::ensure_table_format(conn, table_name,collums)?;

    //CREATE TABLE [name](
    //    HypixelID TEXT NOT NULL UNIQUE,
    //    ID INTEGER PRIMARY KEY AUTOINCREMENT,
    //    Name TEXT
    //);

    check_and_create_lookup_table(conn, lookup_table_name)?;
    
    Ok(())
}

fn check_and_create_lookup_table(conn: &Connection, table_name: &str) -> Result<(), PricingError> {

    // Step 1: Ensure the table exists
    create_table_if_not_exists(conn, table_name)?;

    // Step 2: Validate the table schema
    let columns = fetch_table_schema(conn, table_name)?;

    // Step 3: Handle missing columns (if any)
    if !validate_table_schema(&columns)?{
        ensure_column_exists(conn, table_name, "Name", "TEXT")?;
    }

    Ok(())
}
fn create_table_if_not_exists(conn: &Connection, table_name: &str) -> Result<(), PricingError> {
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
/// returns true if collum exists
fn fetch_table_schema(conn: &Connection, table_name: &str) -> Result<Vec<(String, String, isize)>, PricingError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({});", table_name))?;
    let column_info = stmt.query_map([], |row| {
        Ok((
            row.get::<usize, String>(1)?, // Column name
            row.get::<usize, String>(2)?, // Column type
            row.get::<usize, isize>(5)?,    // Is primary key? (1 if true, 0 if false)
        ))
    })?;
    column_info.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
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
fn ensure_column_exists(conn: &Connection, table_name: &str, column_name: &str, column_type: &str) -> Result<(), PricingError> {
    conn.execute(
    &format!("ALTER TABLE {} ADD COLUMN {} {};", table_name, column_name, column_type),
    [],
    )?;
    
    Ok(())
}

fn get_data_from_api(error_handel: &Sender<ErrorOperation>, context: &Context)-> Result<(BzData,(usize,usize)), PricingError>{
    let data= web_service_adapter::get_data_puls_size(&context.url, 3, Duration::from_secs(3)).map_err(|err|{
        if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), format!("error while feching data from api, retrying nex cycel\n{err}"))){
            return PricingError::ErrorThreadDown(format!("error while feching data from api, retrying nex cycel\n{err}"));
        }
        PricingError::NonFatal}
    );

    let (data,(data_out, _data_in)) = data?;

    let json_data= BzData::from_data(data).map_err(|err|
    match err {
        PricingError::JSONReadError => {if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"error while parsing the json, retrying nex cycel".to_string())){
            return PricingError::ErrorThreadDown("error while parsing the json, retrying nex cycel".to_string());
        }
        PricingError::NonFatal
        },
        PricingError::JSONFormatError(messige) => {if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),format!("error while parsing the json, retrying nex cycel\n{messige}"))){
            return PricingError::ErrorThreadDown(format!("error while parsing the json, retrying nex cycel\n{messige}"));
        }
        PricingError::NonFatal
        },
        _ => err,
    })?;

    if !json_data.success{
        if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"JSON had a unexpexted erro, retrying nex cycel".to_string())){
            return Err(PricingError::ErrorThreadDown("JSON had a unexpexted erro, retrying nex cycel".to_string()));
        }
        return Err(PricingError::NonFatal);
    }

    Ok((json_data,(data_out,data_out)))
}

fn update_index_database(json_data: &BzData, context: &mut Context, error_handel: &Sender<ErrorOperation>) ->Result<(), PricingError>{
    struct Name{
        name:String
    }
    impl SQLformat for Name {
        fn sqlformat(&self) -> Vec<&dyn ToSql> {
            vec![&self.name]
        }
    }
    let hypixel_ids = json_data.products.values()
    .map(|value | Name{name: value.product_id.clone()})
    .collect();

    if data_base_manager::try_write_database(&mut context.conn,hypixel_ids , &context.lookup_table_name, "HypixelID")? > 0{
        if let Err(_) = error_handel.send(ErrorOperation::Print(APP_NAME.to_string(),"new item added to the database, require manual naming.".to_string())){
            return Err(PricingError::ErrorThreadDown("new item added to the database, require manual naming.".to_string()));
        }
    };

    Ok(())
}

fn write_database(context: &mut Context, json_data: BzData, error_handel: &Sender<ErrorOperation>) -> Result<(bool, String), PricingError>{
    struct BazaData{
        id: usize,
        time_stamp: u64,
        sell_price: f64,
        buy_price: f64,
        sell_volme: usize,
        sell_moving_week: usize,
        buy_volme: usize,
        buy_moving_week: usize,
    }
    impl SQLformat for BazaData {
        fn sqlformat(&self) -> Vec<&dyn ToSql> {
            vec![&self.id,&self.time_stamp,&self.sell_price,&self.buy_price,&self.sell_volme,&self.sell_moving_week,&self.buy_volme,&self.buy_moving_week]
        }
    }
    struct Id{
        id: usize
    }
    impl SQLReadable for Id {
        fn from_row(row: &rusqlite::Row) -> Result<Self,DataBaseError> {
            let id=row.get(0)?;
            Ok(Id{id})
        }
    }

    let mut error_send_failed = false;
    let mut error_messig: Vec<String> = Vec::new();
    let time = json_data.last_updated;
    let prices_data =json_data.products.into_values()
    .filter_map(|value| {
        match  data_base_manager::read_database::<Id>(&mut context.conn, &context.lookup_table_name, "ID", &format!("WHERE HypixelID = '{}'", value.product_id)){
            Ok(id) => {
            Some(BazaData{id: id[0].id,
                time_stamp: time,
                sell_price: value.quick_status.sellPrice,
                sell_volme: value.quick_status.sellVolume,
                buy_moving_week: value.quick_status.buyMovingWeek,
                buy_volme: value.quick_status.buyVolume,
                sell_moving_week: value.quick_status.sellMovingWeek,
                buy_price:value.quick_status.buyPrice})
        },
        Err(e) => {
            if let Err(_) =error_handel.send(ErrorOperation::Print(APP_NAME.to_string(), format!("coudind read :\"{}\"\nError: {}", value.product_id,e))){
                error_send_failed = true;
                error_messig.push(format!("coudind read :\"{}\"\nError: {}", value.product_id,e));
            }
            None
        }
        }
        })
        .collect();

    data_base_manager::write_database(&mut context.conn, prices_data, &context.data_table_name, "ID, timeStamp, sellPrice, buyPrice,sellVolume, sellMovingWeek, buyVolume, buyMovingWeek")?;

    Ok((error_send_failed, error_messig.join(";\n")))

}

fn get_bz_data(error_handel: &Sender<ErrorOperation>, context: &mut Context) -> Result<(), PricingError>{
    // get data
    let json_data = get_data_from_api(error_handel, context);
    if let Err(PricingError::NonFatal) = json_data{
        return Ok(());
    }
    let (json_data,(data_out,data_in)) = json_data?;

    // write new item's
    update_index_database(&json_data, context, error_handel)?;

    // write to data base
    let num_items = json_data.products.len();
    let (error_send_failed, error_messig) = write_database(context, json_data, error_handel)?;

    context.update_status(num_items, data_in + data_out)?;
    
    if error_send_failed{
        return Err(PricingError::ErrorThreadDown(error_messig));
    }

    Ok(())
}



#[cfg(test)]
mod tests {
}
