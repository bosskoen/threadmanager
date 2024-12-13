use std::str;
pub use rusqlite::Connection;

use self::helper_functions::*;

pub use self::custom_types::*;

mod custom_types;
mod helper_functions;

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
///     use library::data_base_manager::*;
///     use rusqlite::ToSql;
///     use rusqlite::Connection;
/// 
///     struct Test{
///     id:u64, value1:String, value2: bool
///     }
///     impl SQLformat for Test{ 
///      fn sqlformat(&self) -> Vec<&dyn ToSql>{
///         vec![&self.value2, &self.id, &self.value1]
///         }
///     }
///     let data = vec![Test{id:1,value1: "hello".to_string(), value2: true},
///                     Test{id:2,value1: "world".to_string(), value2: false},
///                     Test{id:5,value1: "cake".to_string(), value2: true}];
///     let mut conn = Connection::open("test.db").unwrap();
/// 
///     write_database(&mut conn, data ,"test", "value2, id, value1");
/// ```
/// 
pub fn write_database<T>(conn: &mut Connection, data: Vec<T>, table_name: &str, table_format: &str) -> Result<(),DataBaseError>
    where T: SQLformat
{
    let placeholders = generate_placeholder(table_format);
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
///     use library::data_base_manager::*;
///     use rusqlite::{Connection, Row};
///     struct User {
///     id: i32, name: String, age: i32,
///     }
///
/// impl SQLReadable for User {
///     fn from_row(row: &Row) -> Result<Self,DataBaseError> {
///         let id = row.get(0)?;
///         let name = row.get(1)?;
///         let age = row.get(2)?;
///         Ok(User {
///             id, name, age
///         })
///     }
/// }
/// 
///     let mut conn = Connection::open("my_database.db").unwrap();
/// 
///     let users: Vec<User> = read_database(&conn, "users", "id, name, age", "WHERE age > 18").unwrap();
/// ```
pub fn read_database<T>(conn: &Connection, table_name: &str, query_column_names: &str, condition: &str) -> Result<Vec<T>,DataBaseError>
where
    T: SQLReadable,
{
    let command = format!(
        "SELECT {} FROM {} {};",
        query_column_names, table_name, condition
    );

    let mut stmt  = conn.prepare(&command)?;
    
    let x = stmt.query_and_then([], |row| T::from_row(row))?
    .collect();
    x
}

///  Deletes an entry from the specified table based on the given condition.
///  give a error if condition is emptie
/// 
/// # Arguments
///
/// - `conn`: A reference to the `rusqlite::Connection` object for the database.
/// - `table_name`: The name of the table to delete from.
/// - `condition`: The SQL condition specifying which rows to delete (e.g., `"id = 1"`).
/// 
/// # Example
/// ``` no_run
///     use library::data_base_manager::*;
///     use rusqlite::Connection;
///     let conn = Connection::open("my_database.db").unwrap();
///     delete_entry(&conn, "users", "id = 1").unwrap();
/// ```
pub fn delete_entry(conn: &Connection, table_name: &str, condition: &str)-> Result<usize,DataBaseError>{
    if condition.trim().is_empty(){
        return Err(DataBaseError::NoConditionFound("Can't delete entire database at once".to_string())); // discriptor
    }
    let command = format!("DELETE FROM {} WHERE {}", table_name,condition);
    Ok(conn.execute(&command, [])?)
}

/// Ensures a SQLite table meets the required format, creating or altering it as necessary.
///
/// # Arguments
/// - `conn`: SQLite database connection.
/// - `table_name`: Name of the table to check or create.
/// - `required_columns`: A vector of tuples (column name, column type, not_null ,is_primary_key).
///
/// # Example
/// ``` no_run
///     use library::{*,data_base_manager::*} ;
///     use rusqlite::Connection;
///     let mut conn = Connection::open("test.db").unwrap();
///     ensure_table_format(&mut conn, "users", vec![
///     define_column!("id", "INTEGER", true ,true), // Primary key
///     define_column!("name", "TEXT",false ,false),
///     define_column!("email", "TEXT",true ,false)
/// ]);
/// ```
pub fn ensure_table_format(
    conn: &mut Connection,
    table_name: &str,
    required_columns: Vec<ColumnDefinition>,
) -> Result<(),DataBaseError> {
    let mut lock_tracaction = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    // Step 1: Check if the table exists and get its current format
    let pragma_query = format!("PRAGMA table_info({});", table_name);

    let mut stmt = lock_tracaction.prepare(&pragma_query)?;

    let pre_existing_columns: Vec<(String, String, bool, bool)> = stmt
    .query_map([], |row| {
        let col_name: String = row.get(1)?; // Column name
        let col_type: String = row.get(2)?; // Column type
        let not_null: bool = row.get(3)?; // is not null
        let is_primary_key: bool = row.get::<_, i32>(5)? != 0; // Is primary key

        Ok((col_name, col_type, not_null, is_primary_key))
    })?
    .collect::<Result<Vec<_>, _>>()?;

    let existing_columns: Vec<ColumnDefinition> = pre_existing_columns.iter()
    .map(|(col_name, col_type, not_null, is_primary_key)| {
        ColumnDefinition::new(col_name, col_type, *not_null, *is_primary_key)
    })
    .collect();

    drop(stmt);
    if existing_columns.is_empty() {
        create_table(&lock_tracaction, &required_columns, table_name)?;
    } else {
        alter_table(&mut lock_tracaction, &required_columns, &existing_columns,table_name)?;
    }
    lock_tracaction.commit()?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection,Row, ToSql};

    #[derive(Debug)]
    struct TestItem {
        id: u64,
        value1: String,
        value2: bool,
    }

    impl SQLformat for TestItem {
        fn sqlformat(&self) -> Vec<&dyn ToSql> {
            vec![&self.value2, &self.id, &self.value1]
        }
    }

    impl SQLReadable for TestItem {
        fn from_row(row: &Row) -> Result<Self, DataBaseError> {
            Ok(TestItem {
                id: row.get(0)?,
                value1: row.get(1)?,
                value2: row.get(2)?,
            })
        }
    }

    #[test]
    fn test_write_database() -> Result<(), DataBaseError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (value2 BOOLEAN, id INTEGER PRIMARY KEY, value1 TEXT);",
            [],
        )?;

        let data = vec![
            TestItem {
                id: 1,
                value1: "Hello".to_string(),
                value2: true,
            },
            TestItem {
                id: 2,
                value1: "World".to_string(),
                value2: false,
            },
        ];

        write_database(&mut conn, data, "test", "value2, id, value1")?;

        let mut stmt = conn.prepare("SELECT id, value1, value2 FROM test ORDER BY id ASC;")?;
        let rows: Vec<TestItem> = stmt
            .query_and_then([], |row| TestItem::from_row(row))?
            .collect::<Result<Vec<_>, _>>()?;

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[0].value1, "Hello");
        assert_eq!(rows[0].value2, true);
        assert_eq!(rows[1].id, 2);
        assert_eq!(rows[1].value1, "World");
        assert_eq!(rows[1].value2, false);

        Ok(())
    }

    #[test]
    fn test_read_database() -> Result<(),DataBaseError> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);",
            [],
        )?;
        conn.execute(
            "INSERT INTO test (id, value1, value2) VALUES (1, 'Hello', 1), (2, 'World', 0);",
            [],
        )?;

        let rows: Vec<TestItem> = read_database(&conn, "test", "id, value1, value2", "WHERE value2 = 1")?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[0].value1, "Hello");
        assert_eq!(rows[0].value2, true);

        Ok(())
    }

    #[test]
    fn test_delete_entry() -> Result<(),DataBaseError> {
        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE test (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);",
            [],
        )?;
        conn.execute(
            "INSERT INTO test (id, value1, value2) VALUES (1, 'Hello', 1), (2, 'World', 0);",
            [],
        )?;

        let deleted_rows = delete_entry(&conn, "test", "id = 1")?;
        assert_eq!(deleted_rows, 1);

        let remaining: Vec<(u64, String, bool)> = conn
            .prepare("SELECT id, value1, value2 FROM test;")?
            .query_map([], |row| {
                Ok((row.get::<_,u64>(0)?, row.get::<_,String>(1)?, row.get::<_, i32>(2)? != 0))
            })?
            .collect::<Result<Vec<_>,_>>()?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].0, 2);
        assert_eq!(remaining[0].1, "World");
        assert_eq!(remaining[0].2, false);

        Ok(())
    }

    #[test]
    fn test_ensure_table_format() -> Result<(),DataBaseError> {
        let mut conn = Connection::open_in_memory()?;

        let columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("value1", "TEXT", false, false),
            ColumnDefinition::new("value2", "BOOLEAN", false, false),
        ];

        ensure_table_format(&mut conn, "test", columns)?;

        // Verify table creation
        let pragma_result: Vec<(String, String)> = conn
            .prepare("PRAGMA table_info('test');")?
            .query_map([], |row| Ok((row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>,_>>()?;
        assert_eq!(pragma_result.len(), 3);
        assert_eq!(pragma_result[0], ("id".to_string(), "INTEGER".to_string()));
        assert_eq!(pragma_result[1], ("value1".to_string(), "TEXT".to_string()));
        assert_eq!(pragma_result[2], ("value2".to_string(), "BOOLEAN".to_string()));

        Ok(())
    }
    #[test]
    fn test_write_database_empty_input() -> Result<(), DataBaseError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute("CREATE TABLE test (value2 BOOLEAN, id INTEGER PRIMARY KEY, value1 TEXT);", [])?;
        let data: Vec<TestItem> = vec![];
        write_database(&mut conn, data, "test", "value2, id, value1")?;
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM test;", [], |row| row.get(0))?;
        assert_eq!(count, 0);
        Ok(())
    }
    #[test]
    fn test_read_database_invalid_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);", []).unwrap();
        let result: Result<Vec<TestItem>, DataBaseError> = read_database(&conn, "test", "nonexistent_column", "");
        assert!(result.is_err());
    }
    #[test]
    fn test_concurrent_ensure_table_format() -> Result<(),DataBaseError> {
        use std::thread;
        use std::path::Path;
        use std::fs;

        // URI for shared in-memory database across connections
        let db_file = "my_database.db"; // File path for the database file
        let mut conn1 = Connection::open(db_file)?;
        let mut conn2 = Connection::open(db_file)?;
    
        // Define the column structure for the table
        let columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("value1", "TEXT", false, false),
            ColumnDefinition::new("value2", "BOOLEAN", false, false),
        ];
    
        let columns2 = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("value1", "TEXT", false, false),
            ColumnDefinition::new("value3", "INTEGER", true, false),
        ];
    
        let handle = thread::spawn(move || {
            ensure_table_format(&mut conn2, "test", columns2).unwrap();
        });
    
        ensure_table_format(&mut conn1, "test", columns)?;
    
        handle.join().unwrap();
    
        // Query the table using conn1 to see the schema and check if value3 exists
        let mut stmt = conn1.prepare("PRAGMA table_info(test);")?;
        let columns_info = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,  // Column name
                row.get::<_, String>(2)?,  // Column type
            ))
        })?;
    
        let columns_info: Vec<(String, String)> = columns_info.collect::<Result<_, _>>()?;
        drop(stmt);
        drop(conn1);

        if Path::new(db_file).exists() {
            fs::remove_file(db_file).unwrap();
        }

        let mut value3_found = false;
        for (name, _) in columns_info {
            if name == "value3" {
                value3_found = true;
            }
        }
    
        // Assert that value3 is included in the schema after conn2 modified it
        assert!(value3_found, "Expected column 'value3' not found in the table");
    
    
        // The columns_info should now include "value3" if conn2 modified the schema
        Ok(())
    }
    #[test]
    fn test_write_database_duplicate_primary_key() -> Result<(), DataBaseError> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);", [])?;
        let data = vec![
            TestItem { id: 1, value1: "First".to_string(), value2: true },
            TestItem { id: 1, value1: "Duplicate".to_string(), value2: false },
        ];
        let result = write_database(&mut conn, data, "test", "value2, id, value1");
        assert!(result.is_err());
        Ok(())
    }
}