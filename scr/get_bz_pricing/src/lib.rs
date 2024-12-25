use std::{fmt:: Display, fs, sync::{atomic::{AtomicBool, Ordering}, mpsc::Sender, Arc, Mutex}, thread, time::{Duration, SystemTime}};
use humansize::{format_size, BINARY};
use library::*;
use data_base_manager::{rusqlite::{self, ToSql}, write_database, ColumnDefinition, Connection, DataBaseError, SQLReadable, SQLformat};
use error_handeler::ErrorOperation;
use parsing::{BzData, Settings};



mod parsing;

struct Context {
    stopflag: AtomicBool,
    status: Arc<Mutex<Box<dyn Status>>>,
    settings_path: String,
    conn: Connection,
    data_table_name: String,
    update_rate: usize ,
    step_rate: usize,
    time_passed: usize,
    last_time_setting_written: SystemTime,
    url: String,
    lookup_table_name: String
}

impl Context {
    fn from(stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<Self, PricingError>{
        let (settings, last_time_setting_written) = Settings::get(&settings_path)?;
        let data_base_path = settings.data_base_path;
        let data_table_name = settings.table_name;
        let lookup_table_name= settings.lookup_table_name;
        
        let mut conn = Connection::open(data_base_path.clone()).map_err(|_|PricingError::DataBaseConnectionError)?;
        validate_data_base(&mut conn, &data_table_name, &lookup_table_name)?;
        initialise_status(&conn, &data_table_name, &status)?;

        Ok(Context{stopflag, status ,settings_path, conn, data_table_name, update_rate: settings.update_rate, step_rate: settings.step_rate, time_passed: 0, last_time_setting_written, url: settings.url, lookup_table_name})
    }

    fn update_timing(&mut self) -> Result<(), PricingError>{
        let mod_time =fs::metadata(&self.settings_path)
            .map_err(|_|PricingError::FileReadError )?
            .modified()
            .map_err(|_| PricingError::FileReadError)?;
        
        let duration_since_last_update = mod_time
            .duration_since(self.last_time_setting_written)
            .map_err(|_| PricingError::WTFError("You time traveled, the file was modified in the past!".to_string()))?;

        if duration_since_last_update.as_secs() <= 0 {
            return Ok(());
        }
        
        let (setting, _) = Settings::get(&self.settings_path)?;
        self.update_rate = setting.update_rate;
        self.step_rate = setting.step_rate;
        self.url = setting.url;
        self.last_time_setting_written = mod_time;
        Ok(())
    }

    fn update_status(&self, items_tracked: usize, data_used: usize) -> Result<(), PricingError>{
        match self.status.lock(){
            Ok(mut stat) => {let internal_statust  = if let Some(mut_status) = (**stat).as_any_mut().downcast_mut::<PricingStatus>(){
                mut_status
            }else{ 
                return Err(PricingError::IncorectStatusType);
            };
            internal_statust.updates_processed += 1;
            internal_statust.items_being_tracked = items_tracked;
            internal_statust.network_data_used += data_used as u64;
            internal_statust.last_update_time = Local::now();
            },
            Err(_) => { 
                return Err(PricingError::LockFailedError);
            },
        }
        Ok(())
    }

}


#[no_mangle]
pub fn start(error_handel: Sender<ErrorOperation>, stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>, settings_path: String) -> Result<(), Box<dyn std::error::Error>>{

    let mut context = Context::from(stopflag, status, settings_path)?;

    loop {
        let start_of_loop = SystemTime::now();
        context.update_timing()?; 
        if context.stopflag.load(Ordering::Relaxed){
            break;
        }
        if context.time_passed >= context.update_rate{
            context.time_passed = 0;
            get_bz_data(&error_handel, &mut context)?;
        }else{
            context.time_passed += context.step_rate;
        }

        let endloop =match start_of_loop.elapsed() {
            Ok(duration) => duration,
            Err(error) => {
                if let Err(_) = error_handel.send(ErrorOperation::Print(format!("error while getting elepsted time: {}", error.to_string()))){
                    return Err(Box::new(PricingError::ErrorThreadDown));
                }
                Duration::ZERO
            },
        };

        if let Some(sleep_duration) = Duration::from_secs(context.step_rate as u64).checked_sub(endloop) {
            thread::sleep(sleep_duration);
        } else {
            if let Err(_) = error_handel.send(ErrorOperation::Print("loop took to long in price logger".to_string())){
                return Err(Box::new(PricingError::ErrorThreadDown));
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
    // Step 1: Check if the table exists, if not, create it
    conn.execute(
        &format!("CREATE TABLE IF NOT EXISTS {}( 
            HypixelID TEXT NOT NULL UNIQUE, 
            ID INTEGER PRIMARY KEY AUTOINCREMENT, 
            Name TEXT
        );", table_name),
        [],
    )?;

    // Step 2: Check the table schema
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({});", table_name))?;
    let column_info = stmt.query_map([], |row| {
        Ok((
            row.get::<usize, String>(1)?, // Column name
            row.get::<usize, String>(2)?, // Column type
            row.get::<usize, i64>(5)?,    // Is primary key? (1 if true, 0 if false)
        ))
    })?;

    let mut has_hypixel_id = false;
    let mut has_id = false;
    let mut has_name = false;
    let mut id_is_primary_key = false;
    let mut hypixel_id_is_unique = false;

    // Collect actual columns and check properties
    for column in column_info {
        let (column_name, column_type, is_primary_key): (String, String, i64) = column?;
        
        match column_name.as_str() {
            "HypixelID" => {
                has_hypixel_id = true;
                if column_type != "TEXT" {
                    let error_message = format!("Error: 'HypixelID' should be of type 'TEXT', found '{}'.", column_type);
                    return Err(PricingError::SQLformatError(error_message));
                }
                if is_primary_key != 0 {
                    let error_message = "Error: 'HypixelID' should not be a primary key.".to_string();
                    return Err(PricingError::SQLformatError(error_message));
                }
                hypixel_id_is_unique = true;
            }
            "ID" => {
                has_id = true;
                if column_type != "INTEGER" {
                    let error_message = format!("Error: 'ID' should be of type 'INTEGER', found '{}'.", column_type);
                    return Err(PricingError::SQLformatError(error_message));
                }
                if is_primary_key == 0 {
                    let error_message = "Error: 'ID' should be the primary key.".to_string();
                    return Err(PricingError::SQLformatError(error_message));
                } else {
                    id_is_primary_key = true;
                }
            }
            "Name" => {
                has_name = true;
                if column_type != "TEXT" {
                    let error_message = format!("Error: 'Name' should be of type 'TEXT', found '{}'.", column_type);
                    return Err(PricingError::SQLformatError(error_message));
                }
            }
            _ => {}
        }
    }

    // Step 3: Validate column structure
    if !has_hypixel_id {
        let error_message = "Error: Column 'HypixelID' is missing.".to_string();
        return Err(PricingError::SQLformatError(error_message));
    }
    if !has_id {
        let error_message = "Error: Column 'ID' is missing.".to_string();
        return Err(PricingError::SQLformatError(error_message));
    }
    if !has_name {
        let error_message = "Error: Column 'Name' is missing.".to_string();
        return Err(PricingError::SQLformatError(error_message));
    }
    if !id_is_primary_key {
        let error_message = "Error: 'ID' should be the primary key.".to_string();
        return Err(PricingError::SQLformatError(error_message));
    }
    if !hypixel_id_is_unique {
        let error_message = "Error: 'HypixelID' should be unique.".to_string();
        return Err(PricingError::SQLformatError(error_message));
    }

    // Step 4: Handle missing columns (if any)
    if !has_name {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN Name TEXT;", table_name),
            [],
        )?;
    }

    Ok(())
}
fn initialise_status(conn: &Connection, table_name: &str,status: &Arc<Mutex<Box<dyn Status>>>) -> Result<(), PricingError>{
    let mut newstatus = PricingStatus::new();
    if let Ok(timestamp) = conn.query_row_and_then(&format!("SELECT max(time) FROM {}", table_name), [], |row| row.get::<_,i64>(0)){
        newstatus.last_update_time = if let Some(local_time) = DateTime::from_timestamp(timestamp, 0) {
            DateTime::from(local_time)
        } else{ newstatus.last_update_time}
    }

    if let Ok(mut status)= status.lock(){
        (*status) = Box::new(newstatus);
    }else {return Err(PricingError::StatusIntialiseError);}
    Ok(())
}

fn get_bz_data(error_handel: &Sender<ErrorOperation>, context: &mut Context) -> Result<(), PricingError>{
    let data= web_service_adapter::get_data_puls_size(&context.url, 3, Duration::from_secs(3)).map_err(|err|{
        if let Err(_) = error_handel.send(ErrorOperation::Print(format!("error while feching data from api, retrying nex cycel\n{}", err))){
            return PricingError::ErrorThreadDown;
        }
        PricingError::NonFatal}
    );

    if let Err(PricingError::NonFatal) = data{
        return Ok(());
    }
    let (data,(data_out, data_in)) = data?;

    let json_data= BzData::from_data(data).map_err(|err|
    match err {
        PricingError::JSONReadError => {if let Err(_) = error_handel.send(ErrorOperation::Print("error while parsing the json, retrying nex cycel".to_string())){
            return PricingError::ErrorThreadDown;
        }
        PricingError::NonFatal
        },
        PricingError::JSONFormatError(messige) => {if let Err(_) = error_handel.send(ErrorOperation::Print(format!("error while parsing the json, retrying nex cycel\n{}", messige))){
            return PricingError::ErrorThreadDown;
        }
        PricingError::NonFatal
        },
        _ => err,
    });

    if let Err(PricingError::NonFatal) = json_data{
        return Ok(());
    }

    let json_data =json_data?;

    if !json_data.success{
        if let Err(_) = error_handel.send(ErrorOperation::Print(format!("JSON had a unexpexted erro,retrying nex cycel", ))){
            return Err(PricingError::ErrorThreadDown);
        }
        return Ok(());
    }

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
        if let Err(_) = error_handel.send(ErrorOperation::Print("new item added to the database, require manual naming.".to_string())){
            return Err(PricingError::ErrorThreadDown);
        }
    }
    
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
    let time = json_data.last_updated;
    let num_item = json_data.products.len();
    let mut error_send_failed = false;
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
            if let Err(_) =error_handel.send(ErrorOperation::Print(format!("coudind read :\"{}\"\nError: {}",value.product_id ,e))){
                error_send_failed = true;
            }
            None
        }
        }
        })
        .collect();

    write_database(&mut context.conn, prices_data, &context.data_table_name, "ID, timeStamp, sellPrice, buyPrice,sellVolume, sellMovingWeek, buyVolume, buyMovingWeek")?;

    context.update_status(num_item, data_in+data_out)?;
    
    if error_send_failed{
        return Err(PricingError::ErrorThreadDown);
    }

    Ok(())
}



#[derive(Debug)]
pub enum PricingError {
    DataBaseConnectionError,
    StatusIntialiseError,
    IncorectStatusType,
    LockFailedError,
    ErrorThreadDown,
    FileReadError,
    TOMLReadError,
    JSONReadError,
    JSONFormatError(String),
    WTFError(String),
    NonFatal,
    SQLformatError(String),
    SQLReadWrite(String),
}

impl Display for PricingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PricingError::DataBaseConnectionError => write!(f, "DATA_BASE_CONNECTION_ERROR: Couldn't get a connection to the given data base."),
            PricingError::StatusIntialiseError => write!(f, "STATUS_INTIALISE_ERROR: Couldn't get a lock on status while initialising."),
            PricingError::IncorectStatusType => write!(f, "INCORECT_STATUS_TYPE: Status wasn't of the corect type."),
            PricingError::LockFailedError => write!(f, "LOCK_FAILED_ERROR: Couldn't get a lock on status while updating."),
            PricingError::ErrorThreadDown => write!(f, "ERROR_THREAD_DOWN: Couldn't send a messige to the error thread."),
            PricingError::TOMLReadError => write!(f, "TOML_READ_ERROR: Error parsing the settings file, i may be malformed or from the wrong appication"),
            PricingError::FileReadError => write!(f, "FILE_READERROR: Coudn't read the settings file."),
            PricingError::WTFError(messig) => write!(f, "good job you dit somthing that sould be imposible:\n{}", messig),
            PricingError::JSONReadError => write!(f, "JSON_READ_ERROR: failed to parse the json file."),
            PricingError::JSONFormatError(cause) => write!(f, "JSON_FORMAT_ERROR: json file didn't have the expected format:\n{}", cause),
            PricingError::NonFatal => write!(f, "a not fatal error that shouldn't be propegated"),
            PricingError::SQLformatError(messig) => write!(f, "SQL_FORMAT_ERROR: {}", messig),
            PricingError::SQLReadWrite(messig) => write!(f, "SQL_READ_WRITE: error while reading or writing to the database{}", messig),
        }
    }
}

impl From<DataBaseError> for PricingError {
    fn from(value: DataBaseError) -> Self {
       PricingError::SQLReadWrite(format!("{:?}",value))
    }
}
impl From<rusqlite::Error> for PricingError {
    fn from(value: rusqlite::Error) -> Self {
        PricingError::SQLformatError(format!("{:?}", value))
    }
}
impl std::error::Error for PricingError{}

pub struct PricingStatus{
    pub updates_processed: usize,
    pub items_being_tracked: usize,
    pub network_data_used: u64,
    pub last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PricingStatus {
    pub fn new() ->Self{
        PricingStatus{updates_processed:0,items_being_tracked:0, network_data_used:0, last_update_time: Local::now() , start_time: Local::now()}
    }
}

impl_status!{PricingStatus, |s: &PricingStatus| format!{
    "Pricing tracker processed {} updates.\n\
    Currently tracking {} items.\n\
    Total network data used: {}.\n\
    Last update was at {}.\n\
    Thread started at {} and has been running for {}.",
    s.updates_processed,
    s.items_being_tracked,
    format_size(s.network_data_used, BINARY),
    s.last_update_time.format("%Y %m-%d; %H:%M:%S"),
    s.start_time.format("%Y %m-%d; %H:%M:%S"),
    format_duration(s.start_time, Local::now())
}}

#[cfg(test)]
mod tests {
}
