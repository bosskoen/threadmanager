use std::fmt;
use sqlx::Error;
pub use sqlx::postgres::PgRow;

use crate::data_base_manager::normalize_identifier;



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

    pub fn base_type(&self) -> PostgresType {
        match self {
            PostgresType::i16Auto => PostgresType::i16,
            PostgresType::i32Auto => PostgresType::i32,
            PostgresType::i64Auto => PostgresType::i64,
            _ => *self,
        }
    }
}



pub fn can_add_column_to_non_empty_table(col: &ColumnDefinition) -> bool {
    if !col.not_null {
        return true; // NULLs are allowed
    }

    if col.auto_increment && !col.primary_key {
        return true; // SERIAL/BIGSERIAL types auto-fill via nextval()
    }

    if col.default.is_none() {
        return false; // NOT NULL without default is unsafe
    }

    // If it's primary or must be unique and not null, ensure default is unique
    if !col.primary_key && col.unique && col.not_null {
        if let Some(default) = &col.default {
            let lowered = default.to_lowercase();
            if lowered.contains("nextval(") {
                return true; // Sequence-based default is safe
            } else {
                return false; // Default might not be unique
            }
        }
    }

    // Otherwise, default exists and no strong uniqueness constraint
    true
}

/// Definition of a table column used to create or compare schema elements.
///
/// Each column definition includes type, constraints (e.g., not null, unique),
/// optional default values, and whether it uses an auto-incrementing type.
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    /// Name of the column in the table.
    name: String,

    /// PostgreSQL type of the column, including auto-incrementing variants.
    col_type: PostgresType,

    /// Whether the column must not contain NULL values.
    not_null: bool,

    /// Whether this column is the primary key of the table.
    primary_key: bool,

    /// Whether this column must have unique values.
    unique: bool,

    /// Optional default value as a raw SQL string.
    default: Option<String>,

    /// Whether the column is auto-incrementing (e.g., `SERIAL`, `BIGSERIAL`).
    auto_increment: bool,
}

impl<'a> ColumnDefinition {
 /// Create a new column definition.
    ///
    /// If `primary_key` is true, `not_null` and `is_unique` are forced to `true`.
    /// If `col_type` is an auto-increment type (`i16Auto`, `i32Auto`, `i64Auto`),
    /// `not_null` is forced to `true`, and any `default` is set to None.
    ///
    /// # Arguments
    /// - `name`: The column name.
    /// - `col_type`: The column's PostgreSQL type.
    /// - `not_null`: Whether the column disallows NULL values.
    /// - `primary_key`: Whether the column is a primary key.
    /// - `is_unique`: Whether the column should be marked as `UNIQUE`.
    /// - `default`: Optional default value expression.
    ///
    /// # Example
    /// ```no_run
    /// use library::data_base_manager::*;
    /// let col = ColumnDefinition::new(
    ///     "id".to_string(),
    ///     PostgresType::i64Auto,
    ///     false,
    ///     true,
    ///     false,
    ///     None
    /// );
    /// ```
    pub fn new(name: String, col_type: PostgresType, not_null: bool, primary_key: bool, is_unique: bool, default: Option<String>) -> Self {
    let mut not_null = not_null;
        let mut unique = is_unique;
        let mut default = default;

        if primary_key {
            not_null = true;    // Primary keys must be NOT NULL
            unique = true;  // Primary keys are already UNIQUE
        }

        let auto_increment = match col_type {
            PostgresType::i16Auto | PostgresType::i32Auto | PostgresType::i64Auto => {
                not_null = true;
                default = None; // Let SERIAL/BIGSERIAL use implicit sequence
                true
            }
            _ => {
                if let Some(ref def) = default {
                    def.to_lowercase().contains("nextval(")
                } else {
                    false
                }
            }
        };

        Self {
            name: normalize_identifier(&name),
            col_type,
            not_null,
            primary_key,
            unique,
            default,
            auto_increment
        }
    }

    /// Converts the column definition into a SQL column fragment.
    ///
    /// The result includes type, constraints, and optional default clause.
    /// Suitable for use in `CREATE TABLE` or `ALTER TABLE ADD COLUMN`.
    ///
    /// # Example
    /// ```
    /// use library::data_base_manager::*;
    /// let col = ColumnDefinition::new(
    ///     "username".to_string(),
    ///     PostgresType::String,
    ///     true,
    ///     false,
    ///     true,
    ///     Some("'guest'".to_string())
    /// );
    /// assert_eq!(
    ///     col.to_sql(true),
    ///     "username TEXT UNIQUE NOT NULL DEFAULT 'guest'"
    /// );
    /// ```
    pub fn to_sql(&self, is_single_pk: bool) -> String {
        let mut sql = format!("{} {}", self.name, self.col_type.to_sql_type());

        if self.primary_key { 
            if is_single_pk{ sql.push_str(" PRIMARY KEY");}
        }else{
            if self.unique {
                sql.push_str(" UNIQUE");
            }
            if self.not_null {
            sql.push_str(" NOT NULL");
            }
        }
        if self.col_type != PostgresType::i16Auto
            && self.col_type != PostgresType::i32Auto
            && self.col_type != PostgresType::i64Auto
        {
            if let Some(ref default) = self.default {
            sql.push_str(&format!(" DEFAULT {}", default));
            }
        }
        sql
    }
    
/// Compares two column definitions for logical schema equality.
    ///
    /// This comparison accounts for:
    /// - Exact name match
    /// - Type match (or both are auto-increment and same base type)
    /// - Identical nullability, primary key, uniqueness, and auto-increment flags
    /// - Same default expression (or both are `None`)
    ///
    /// # Example
    /// ```no_run
    /// use library::data_base_manager::*;
    /// let a = ColumnDefinition::new(
    ///     "id".to_string(),
    ///     PostgresType::i32Auto,
    ///     true,
    ///     true,
    ///     true,
    ///     None
    /// );
    /// let b = ColumnDefinition::new(
    ///     "id".to_string(),
    ///     PostgresType::i64Auto,
    ///     true,
    ///     true,
    ///     true,
    ///     None
    /// );
    /// assert!(a.are_same(&b)); // Same base type and both auto-increment
    /// ```
pub fn are_same(&self, other: &ColumnDefinition) -> bool {
    self.name == other.name &&
    (self.col_type == other.col_type || (
        self.auto_increment &&
        other.auto_increment &&
        self.col_type.base_type() == other.col_type.base_type()
    )) &&
    self.not_null == other.not_null &&
    self.primary_key == other.primary_key &&
    self.unique == other.unique &&
    self.auto_increment == other.auto_increment &&
    match (&self.default, &other.default) {
        (Some(a), Some(b)) => a == b,
        (None, None) => true,
        _ => false,
    }
}

    /// Get the column name.
    pub fn name(&self) -> &String {
        &self.name
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
    pub fn primary_key(&self) -> bool {
        self.primary_key
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PgSequencePermission{
    All,
    Update,
    Select,
    Usage,
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
    SchemaMismatchDetails(String),
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
            DataBaseError::SchemaMismatchDetails(msg) => write!(f, "conflicting columns:\n{}", msg),
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