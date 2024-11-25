use std::str;
use rusqlite::Connection;


pub fn get_database_connection(path: &str) -> Connection{
    match Connection::open(path) {
        Ok(con) => con,
        Err(err) => todo!("error with feshing data base{}",err),
    }
}

pub fn write_database(conn: Connection, data: Vec<Box<dyn SQLformat>>, table_name: &str, table_fromat: &str, table_data_count: &str){
    let mut command: String = format!("INSERT INTO {} {} VALUES {}", table_name, table_fromat, table_data_count);
    let values : Vec<String> = data.iter().map(|pice|{pice.sqlfarmat()}).collect();
    command.push_str(&values.join(","));
    command.push_str(";");
    conn.execute(&command, params)
}

pub trait SQLformat {
    fn sqlfarmat(&self) -> String;
}