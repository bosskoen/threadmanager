use std::str;
use rusqlite::{Connection, Row, ToSql};


pub fn get_database_connection(path: &str) -> Connection{
    match Connection::open(path) {
        Ok(con) => con,
        Err(err) => todo!("error with feshing data base{}",err),
    }
}

pub fn write_database(conn: &mut Connection, data: Vec<Box<dyn SQLformat>>, table_name: &str, table_format: &str){
    //TODO check op tabel bestaat
    let placeholders = table_format
    .split(',')
    .map(|_| "?")
    .collect::<Vec<_>>()
    .join(", ");
    let mut command: String = format!("INSERT INTO {} ({}) VALUES ({})", table_name, table_format, placeholders);

    let transaction = if let Ok(trans)= conn.transaction(){
        trans
    }else{
        todo!("impement error handel");
    };
    
    for piece in data {
        transaction.execute(&command, &piece.sqlformat()[..]);  //TODO ERROR handel
    }
    
    transaction.commit();
}

pub trait SQLReadable: Sized {
    /// Define how to construct a struct from a row.
    fn from_row(row: &Row) -> Self;
}

pub fn read_database<T>(conn: &mut Connection, table_name: &str, query_column_names: &str, condition: &str) -> Vec<T>
where
    T: SQLReadable,
{
    //TODO check of alles bestaat
    // Create the SQL query dynamically
    let command = format!(
        "SELECT {} FROM {} {};", 
        query_column_names, 
        table_name, 
        condition
    );

    // Prepare the SQL statement
    let mut stmt = if let Ok(statment) = conn.prepare(&command){
        statment
    }else {
        todo!("impement error handel");     //TODO error
    };

    let mut result: Vec<T> = Vec::new();
    // Execute the query and map each row to a custom struct
    let rows = stmt.query_map([], |row| {result.push(T::from_row(row)); 
        Ok(())
    });

    result
}

delete || add

pub trait SQLformat {
    fn sqlformat(&self) -> Vec<&dyn ToSql>;
}