use std::fmt;
use sqlx::Error;
pub use sqlx::postgres::PgRow;

/// Macro to define a column for a table schema.
/// 
/// # Example
/// ```
/// use library::define_column;
/// use library::data_base_manager::PostgresType;
/// use library::data_base_manager::ColumnDefinition;
/// let col = define_column!("id", PostgresType::i32, true, true);
/// ```
#[macro_export]
macro_rules! define_column {
    ($name:expr, $col_type:expr, $not_null:expr, $is_primary_key:expr) => {
        ColumnDefinition::new($name, $col_type, $not_null, $is_primary_key)
    };
}

/// Macro to implement [`SQLReadable`] for a struct.
/// 
/// # Example
/// ```ignore
/// impl_sql_readable!(User { id: i32, name: String, active: bool });
/// ```
#[macro_export]
macro_rules! impl_sql_readable {
    ($struct_name:ident { $( $field:ident : $type:ty ),* }) => {
        impl SQLReadable for $struct_name {
            fn from_row(row: &PgRow) -> Result<Self, DataBaseError> {
                Ok($struct_name {
                    $(
                        $field: library::data_base_manager::get_colum_value(row, stringify!($field))?,
                    )*
                })
            }
        }
    };
}

/// Macro to implement [`SQLformat`] for a struct.
/// 
/// # Example
/// ```ignore
/// impl_sql_format!(User { id, name, active });
/// ```
#[macro_export]
macro_rules! impl_sql_format {
    ($struct_name:ident { $( $field:ident ),* }) => {
        impl<'c> SQLformat<'c> for $struct_name {
            fn sqlformat(&'c self) -> Vec<ToSql<'c>> {
                vec![
                    $(
                        to_sql_variant(&self.$field),
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
/// use sqlx::postgres::PgRow;
/// use library::data_base_manager::{SQLReadable, DataBaseError, get_colum_value};
///
/// struct User {
///     id: i32,
///     name: String,
///     active: bool,
/// }
///
/// impl SQLReadable for User {
///     fn from_row(row: &PgRow) -> Result<Self, DataBaseError> {
///         Ok(User {
///             id: get_colum_value(row, "id")?,
///             name: get_colum_value(row, "name")?,
///             active: get_colum_value(row, "active")?,
///         })
///     }
/// }
/// ```
pub trait SQLReadable: Sized {
    /// Construct a struct from a database row.
    fn from_row(row: &PgRow) -> Result<Self, DataBaseError>;
}

/// Trait for formatting a struct as a vector of [`ToSql`] values for SQL insertion/updating.
/// 
/// # Example
/// ```
/// use library::data_base_manager::SQLformat;
/// use library::data_base_manager::ToSql;
/// struct Test {
///     id: i64,
///     value1: String,
///     value2: bool,
/// }
/// impl<'c> SQLformat<'c> for Test {
///     fn sqlformat(&'c self) -> Vec<ToSql<'c>> {
///         vec![ToSql::i64(self.id), ToSql::Text(&self.value1), ToSql::Bool(self.value2)]
///     }
/// }
/// ```
pub trait SQLformat<'c> {
    /// Convert the struct into a vector of [`ToSql`] values.
    fn sqlformat(&'c self) -> Vec<ToSql<'c>>;
}

/// Enum representing supported SQL value types for binding to queries.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ToSql<'a> {
    i32(i32),
    i64(i64),
    i16(i16),
    f32(f32),
    f64(f64),
    Text(&'a str),
    Bool(bool),
}

impl ToSql<'_> {
    /// Bind this value to a SQLx query.
    pub fn bind<'q>(
        self,
        query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
        match self {
            ToSql::i32(val) => query.bind(val),
            ToSql::i64(val) => query.bind(val),
            ToSql::i16(val) => query.bind(val),
            ToSql::f32(val) => query.bind(val),
            ToSql::f64(val) => query.bind(val),
            ToSql::Text(val) => query.bind(val.to_string()),
            ToSql::Bool(val) => query.bind(val),
        }
    }
}

/// Enum representing supported PostgreSQL column types.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostgresType {
    i32,      // INT/INTEGER
    i64,      // BIGINT
    i16,      // SMALLINT
    f32,      // REAL
    f64,      // DOUBLE PRECISION
    bool,     // BOOLEAN
    String,   // TEXT
    i16Auto,  // SMALLSERIAL
    i32Auto,  // SERIAL
    i64Auto,  // BIGSERIAL
}

impl PostgresType {
    /// Get the SQL type as a string.
    pub fn to_sql_type(&self) -> &str {
        match self {
            PostgresType::i32 => "INTEGER",
            PostgresType::i64 => "BIGINT",
            PostgresType::i16 => "SMALLINT",
            PostgresType::f32 => "REAL",
            PostgresType::f64 => "DOUBLE PRECISION",
            PostgresType::bool => "BOOLEAN",
            PostgresType::String => "TEXT",
            PostgresType::i16Auto => "SMALLSERIAL",
            PostgresType::i32Auto => "SERIAL",
            PostgresType::i64Auto => "BIGSERIAL",
        }
    }

    /// Parse a SQL type string and auto-increment flag into a [`PostgresType`].
    pub fn from_sql_type(sql: &str, is_auto: bool) -> Option<Self> {
        match (sql.to_uppercase().as_str(), is_auto) {
            ("SMALLINT", false) => Some(PostgresType::i16),
            ("INTEGER", false) | ("INT", false) => Some(PostgresType::i32),
            ("BIGINT", false) => Some(PostgresType::i64),
            ("REAL", _) => Some(PostgresType::f32),
            ("DOUBLE PRECISION", _) => Some(PostgresType::f64),
            ("BOOLEAN", _) => Some(PostgresType::bool),
            ("TEXT", _) => Some(PostgresType::String),
            ("SMALLINT", true) => Some(PostgresType::i16Auto),
            ("INTEGER", true) => Some(PostgresType::i32Auto),
            ("BIGINT", true) => Some(PostgresType::i64Auto),
            _ => None,
        }
    }
}

/// Definition of a table column.
pub struct ColumnDefinition<'a> {
    name: &'a str,
    col_type: PostgresType,
    not_null: bool,
    is_primary_key: bool,
}

impl<'a> ColumnDefinition<'a> {
    /// Create a new column definition.
    pub fn new(name: &'a str, col_type: PostgresType, not_null: bool, is_primary_key: bool) -> Self {
        Self { name, col_type, not_null, is_primary_key }
    }

    /// Get the column name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Get the column type.
    pub fn col_type(&self) -> PostgresType {
        self.col_type
    }

    /// Whether the column is NOT NULL.
    pub fn not_null(&self) -> bool {
        self.not_null
    }

    /// Whether the column is a primary key.
    pub fn is_primary_key(&self) -> bool {
        self.is_primary_key
    }
}

/// Convert a value to a [`ToSql`] variant using [`ToSqlConvert`].
pub fn to_sql_variant<'a, T>(value: &'a T) -> ToSql<'a>
where
    T: ToSqlConvert<'a>,
{
    T::to_sql(value)
}

/// Trait for converting a value to a [`ToSql`] variant.
pub trait ToSqlConvert<'a> {
    /// Convert the value to a [`ToSql`] variant.
    fn to_sql(value: &'a Self) -> ToSql<'a>;
}

// Implementations for common types:
impl<'a> ToSqlConvert<'a> for str {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::Text(value)
    }
}
impl<'a> ToSqlConvert<'a> for String {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::Text(value)
    }
}
impl<'a> ToSqlConvert<'a> for f64 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::f64(*value)
    }
}
impl<'a> ToSqlConvert<'a> for f32 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::f32(*value)
    }
}
impl<'a> ToSqlConvert<'a> for u64 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i64(*value as i64)
    }
}
impl<'a> ToSqlConvert<'a> for u32 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i64(*value as i64)
    }
}
impl<'a> ToSqlConvert<'a> for u16 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i32(*value as i32)
    }
}
impl<'a> ToSqlConvert<'a> for u8 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i16(*value as i16)
    }
}
impl<'a> ToSqlConvert<'a> for i64 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i64(*value)
    }
}
impl<'a> ToSqlConvert<'a> for i32 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i32(*value)
    }
}
impl<'a> ToSqlConvert<'a> for i16 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i16(*value)
    }
}
impl<'a> ToSqlConvert<'a> for i8 {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::i16(*value as i16)
    }
}
impl<'a> ToSqlConvert<'a> for bool {
    fn to_sql(value: &'a Self) -> ToSql<'a> {
        ToSql::Bool(*value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgPermission {
    Select,
    Insert,
    Update,
    Delete,
    Truncate,
    References,
    Trigger,
    All,
}
pub struct UserPermissions {
    pub table: String,
    pub user: String,
    pub permissions: Vec<PgPermission>,
}

/// Custom error type for database operations.
#[derive(Debug)]
pub enum DataBaseError {
    /// Error from the underlying database driver.
    DataBaseError(Error),
    /// No condition found for a query.
    NoConditionFound(String),
    /// Invalid condition in a query.
    InvalidCondition(String),
    /// Error when altering a table.
    AlterTableError(String),
    /// Error related to Tokio runtime.
    TokioError(String),
    /// Unknown column type returned from the database.
    UnknownColumnType(String),
    UnknownPermision(String),
}

impl fmt::Display for DataBaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataBaseError::DataBaseError(err) => write!(f, "Database Error: {}", err),
            DataBaseError::NoConditionFound(msg) => write!(f, "No Condition Found: {}", msg),
            DataBaseError::AlterTableError(msg) => write!(f, "Tried altering a table but got this error:\n{}", msg),
            DataBaseError::InvalidCondition(msg) => write!(f, "Invalid Condition: {}", msg),
            DataBaseError::TokioError(msg) => write!(f, "Tokio Runtime Error: {}", msg),
            DataBaseError::UnknownColumnType(msg) => write!(f, "Could not resolve type returned from database: ({msg})"),
            DataBaseError::UnknownPermision(msg) => write!(f, "Unknown permission error: {}", msg),
    }
}
}

impl std::error::Error for DataBaseError {}

impl From<Error> for DataBaseError {
    fn from(err: Error) -> DataBaseError {
        DataBaseError::DataBaseError(err)
    }
}