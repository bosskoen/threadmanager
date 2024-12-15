use std::{any::Any, sync::{atomic::AtomicBool, mpsc::Sender, Arc, Mutex}};
use library::*;
use data_base_manager::Connection;
use error_handeler::{ErrorOperation, RGB};

use humansize::{format_size,BINARY};

pub fn start(error_handel: Sender<ErrorOperation>, stopflag: AtomicBool, status: Arc<Mutex<Box<dyn Status>>>) -> usize{
    let path = String::new(); //TODO all info from JSON setings file
    let table_name = String::new(); //TODO JSON
    let mut conn = if let Ok( conn) = Connection::open(path){
        conn
    } else{
        throw_exit_error!(error_handel, PrintAndChangeLed("coudn't get a connection to the given data base",RGB::from_hex(0xff_00_00)), 2);
    };

    if let Err(code) = initialise_status(&conn, &table_name, status) {
        throw_exit_error!(error_handel, PrintAndChangeLed("coudn't change status to custom type", RGB::from_hex(0xff_00_00)), 2);
    }

    return 0;
}

fn initialise_status(conn: &Connection, table_name: &str,status: Arc<Mutex<Box<dyn Status>>>) -> Result<usize, usize>{
    let mut newstatus = PrisingStatus::new();
    if let Ok(timestamp) = conn.query_row_and_then(&format!("SELECT max(time) FROM {}", table_name), [], |row| row.get::<_,i64>(0)){
        newstatus.last_update_time = if let Some(local_time) = DateTime::from_timestamp(timestamp, 0) {
            DateTime::from(local_time)
        } else{ newstatus.last_update_time}
    };

    if let Ok(mut status)= status.lock(){
        (*status) = Box::new(newstatus);
    }else {return Err(3);}

    Ok(0)
}
struct PrisingStatus{
    updates_processed: usize,
    items_being_tracked: usize,
    network_data_used: u64,
    last_update_time: DateTime<Local>,
    start_time: DateTime<Local>
}

impl PrisingStatus {
    fn new() ->Self{
        PrisingStatus{updates_processed:0,items_being_tracked:0, network_data_used:0, last_update_time: Local::now() , start_time: Local::now()}
    }
}

impl_status!{PrisingStatus, |s: &PrisingStatus| format!{
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
