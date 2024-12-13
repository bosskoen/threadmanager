use std::fmt;

use rusqlite::{Error as SqliteError, Row, ToSql};

#[macro_export]
macro_rules! define_column {
    ($name:expr, $col_type:expr, $not_null:expr, $is_primary_key:expr) => {
        ColumnDefinition::new($name, $col_type, $not_null, $is_primary_key)
    };
}

#[macro_export]
macro_rules! impl_sql_readable {
    ($struct_name:ident { $( $field:ident : $type:ty ),* }) => {
        impl SQLReadable for $struct_name {
            fn from_row(row: &Row) -> Result<Self, DataBaseError> {
                Ok($struct_name {
                    $(
                        $field: row.get(stringify!($field))?,
                    )*
                })
            }
        }
    };
}

#[macro_export]
macro_rules! impl_sql_format {
    ($struct_name:ident { $( $field:ident ),* }) => {
        impl SQLformat for $struct_name {
            fn sqlformat(&self) -> Vec<&dyn ToSql> {
                vec![
                    $(
                        &self.$field as &dyn ToSql,
                    )*
                ]
            }
        }
    };
}


/// A trait for constructing a struct from a database row.
/// 
/// # Example
/// ```
///     use rusqlite::Row;
///     use library::data_base_manager::{SQLReadable,DataBaseError};
/// 
///     struct User {
///     id: i32, name: String, age: i32,
/// }
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
/// ```
pub trait SQLReadable: Sized {
    /// Define how to construct a struct from a row.
    fn from_row(row: &Row) -> Result<Self,DataBaseError>;
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

pub struct ColumnDefinition<'a>{
    name: &'a str,
    col_type: &'a str,
    not_null: bool,
    is_primary_key: bool
}

impl<'a> ColumnDefinition<'a>{
    pub fn new(name: &'a str, col_type: &'a str, not_null: bool, is_primary_key: bool) -> Self {
        Self { name, col_type, not_null, is_primary_key }
    }

    pub fn name(&self) -> &str{
        self.name
    }

    pub fn col_type(&self) -> &str{
        self.col_type
    }
    pub fn not_null(&self) -> bool{
        self.not_null
    }
    pub fn is_primary_key(&self) -> bool{
        self.is_primary_key
    }
}

#[derive(Debug)]
pub enum DataBaseError {
    Sqlite(SqliteError), // SQLite error will be directly part of our custom error
    NoConditionFound(String),        // Other types of custom errors
    AlterTableError(String)
}

// Implement `fmt::Display` to print errors
impl fmt::Display for DataBaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataBaseError::Sqlite(err) => write!(f, "SQLite Error: {}", err),
            DataBaseError::NoConditionFound(msg) => write!(f, "No Condition Found: {}", msg),
            DataBaseError::AlterTableError(msg) => write!(f, "Tryed altering a tabe but got this error:\n{}", msg),
        }
    }
}

// Optionally implement `std::error::Error`
impl std::error::Error for DataBaseError {}

impl From<SqliteError> for DataBaseError {
    fn from(err: SqliteError) -> DataBaseError {
        DataBaseError::Sqlite(err)  // Directly convert to MyError without nesting
    }
}