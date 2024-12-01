use std::str;
use rusqlite::{Error, Connection, Row, ToSql };


/// a simpel fucntion to write to a SQLite database.
/// this function doesn't check if the table or colums esits
/// 
/// # Arguments
/// - `conn`: SQLite database connection.
/// - `table_name`: Name of the table to check or create.
/// - `data` : a vector that implements SQLformat.
/// - `table_format` : a coma seperated string of colum names to tell this function were an how to write
/// 
/// # Example
/// ``` no_run
///     struct test{
///     id:u64, value1:Sting, value2: bool
///     }
///     impl SQLformat for test{ 
///      fn sqlformat(&self) -> Vec<&dyn ToSql>{
///         vec![&self.value2, &self.id, &self.value2]
///         }
///     }
///     let data = vec![test{id:1,value1: "hello".to_string(), value2: true},
///                     test{id:2,value1: "world".to_string(), value2: false},
///                     test{id:5,value1: "cake".to_string(), value2: true}]
///     let mut conn = Connection::open("test.db").unwrap();
/// 
///     write_database(conn, data ,"test", "value2, id, value1");
/// ```
/// 
pub fn write_database<T>(conn: &mut Connection, data: Vec<T>, table_name: &str, table_format: &str) -> Result<(),Error>
    where T: SQLformat
{
    let placeholders = table_format
    .split(',')
    .map(|_| "?")
    .collect::<Vec<_>>()
    .join(", ");
    let command: String = format!("INSERT INTO {} ({}) VALUES ({})", table_name, table_format, placeholders);

    let transaction = conn.transaction()?;
    {
        let mut stmt = transaction.prepare(&command)?;
        for piece in data {
            stmt.execute( &piece.sqlformat()[..])?; 
        }
    }
    transaction.commit()?;
    Ok(())
}
/// A trait for constructing a struct from a database row.
/// 
/// # Example
/// ```
///     use rusqlite::Row;
///     use library::data_base_manager::SQLReadable;
/// 
///     struct User {
///     id: i32, name: String, age: i32,
/// }
/// impl SQLReadable for User {
///     fn from_row(row: &Row) -> Result<Self,Error> {
///         let id = row.get(0)?;
///         let name = row.get(1)?;
///         let age = row.get(2)?;
///         User {
///             id, name, age
///         }
///     }
/// }
/// ```
pub trait SQLReadable: Sized {
    /// Define how to construct a struct from a row.
    fn from_row(row: &Row) -> Result<Self,Error>;
}


/// a simpel function to read a SQLite database
/// this fuction doesn't check if your qerry is valid
/// 
/// # Arguments
///
/// - `conn`: SQLite database connection
/// - `table_name`: The name of the database table to query.
/// - `query_column_names`: A comma-separated string of column names to select.
/// - `condition`: A string representing the condition for the SQL query,
///   such as `"WHERE id = 1"`. Use an empty string if no condition is needed.
///
/// # Example
/// ``` no_run
///     use rusqlite::Connection;
///     struct User {
///     id: i32, name: String, age: i32,
///     }
///
/// impl SQLReadable for User {
///     fn from_row(row: &Row) -> Result<Self,Error> {
///         let id = row.get(0)?;
///         let name = row.get(1)?;
///         let age = row.get(2)?;
///         User {
///             id, name, age
///         }
///     }
/// }
/// 
///     let mut conn = Connection::open("my_database.db").unwrap();
/// 
///     let users: Vec<User> = read_database(conn, "users", "id, name, age", "WHERE age > 18");
/// ```
pub fn read_database<T>(conn: &Connection, table_name: &str, query_column_names: &str, condition: &str) -> Result<Vec<T>,Error>
where
    T: SQLReadable,
{
    let command = format!(
        "SELECT {} FROM {} {};", 
        query_column_names, 
        table_name, 
        condition
    );

   
    let mut stmt  = conn.prepare(&command)?;

    let mut result: Vec<T> = Vec::new();
    
    stmt
    .query_map([], |row|
        T::from_row(row))
        .and_then(|mapped_rows| {
            mapped_rows.collect::<Result<Vec<T>, _>>() // Collect rows into Vec<T>
        }); //WHYYYYYY
 
    Ok(result)
}

///  Deletes an entry from the specified table based on the given condition.
///  doesn't do enithing if condition is emptie
/// 
/// # Arguments
///
/// - `conn`: A reference to the `rusqlite::Connection` object for the database.
/// - `table_name`: The name of the table to delete from.
/// - `condition`: The SQL condition specifying which rows to delete (e.g., `"id = 1"`).
/// 
/// # Example
/// ``` no_run
///     use rusqlite::Connection;
///     let conn = Connection::open("my_database.db").unwrap();
///     delete_entry(&conn, "users", "id = 1").unwrap();
/// ```
pub fn delete_entry(conn: &Connection, table_name: &str, condition: &str){
    if condition.trim().is_empty(){
        panic!("Condition cannot be empty to prevent accidental deletion of all rows."); //TODO error delete alles?
    }
    let command = format!("DELETE FROM {} WHERE {}", table_name, condition);
    conn.execute(&command, []);
}

/// Ensures a SQLite table meets the required format, creating or altering it as necessary.
///
/// # Arguments
/// - `conn`: SQLite database connection.
/// - `table_name`: Name of the table to check or create.
/// - `required_columns`: A vector of tuples (column name, column type, is_primary_key).
///
/// # Example
/// ``` no_run
///     use rusqlite::Connection;
///     let conn = Connection::open("test.db").unwrap();
///     ensure_table_format(&conn, "users", vec![
///     ("id", "INTEGER", true), // Primary key
///     ("name", "TEXT", false),
///     ("email", "TEXT", false)
/// ]);
/// ```
pub fn ensure_table_format(
    conn: &Connection,
    table_name: &str,
    required_columns: Vec<(&str, &str, bool)>,
) -> Result<()> {
    // Step 1: Check if the table exists and get its current format
    let pragma_query = format!("PRAGMA table_info({});", table_name);
    let mut stmt = conn.prepare(&pragma_query)?;
    let existing_columns: Vec<(String, String, bool)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?, // Column name
                row.get::<_, String>(2)?, // Column type
                row.get::<_, i32>(5)? != 0, // Is primary key (pk column != 0)
            ))
        })?
        .filter_map(Result::ok)
        .collect();

    if existing_columns.is_empty() {
        // Table doesn't exist, create it with required columns
        let columns_def = required_columns
            .iter()
            .map(|(name, col_type, is_pk)| {
                if *is_pk {
                    format!("{} {} PRIMARY KEY", name, col_type)
                } else {
                    format!("{} {}", name, col_type)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let create_table_query = format!("CREATE TABLE {} ({});", table_name, columns_def);
        conn.execute(&create_table_query, [])?;
        println!("Table '{}' created.", table_name);
    } else {
        // Step 2: Add missing columns if the table exists
        for (name, col_type, is_pk) in required_columns {
            if !existing_columns.iter().any(|(col_name, col_type_existing, is_pk_existing)| {
                col_name == name && col_type_existing == col_type && is_pk == *is_pk_existing
            }) {
                if is_pk {
                    println!(
                        "Cannot add primary key column '{}' to existing table '{}'.",
                        name, table_name
                    );
                } else {
                    let alter_table_query =
                        format!("ALTER TABLE {} ADD COLUMN {} {};", table_name, name, col_type);
                    conn.execute(&alter_table_query, [])?;
                    println!(
                        "Added column '{}' of type '{}' to table '{}'.",
                        name, col_type, table_name
                    );
                }
            }
        }
    }

    Ok(())
}

/// #Example
/// ```
///     use rusqlite::ToSql;
///     use library::data_base_manager::SQLformat;
/// 
///     struct Test{
///     id:u64, value1:String, value2: bool
///     }
///     impl SQLformat for Test{ 
///      fn sqlformat(&self) -> Vec<&dyn ToSql>{
///         vec![&self.id, &self.value1, &self.value2]
///         }
///     }
/// ```
pub trait SQLformat {
    fn sqlformat(&self) -> Vec<&dyn ToSql>;
}