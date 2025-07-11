mod custom_types;
pub use custom_types::*;
mod async_connection;
pub use async_connection::AsyncConnection;
pub use sqlx;

use sqlx::{Decode, Error, PgPool, Postgres, Row, Type};
use tokio::runtime::Runtime;

pub struct SyncConnection {
    conn: AsyncConnection,
    tokio: tokio::runtime::Runtime,
}

impl SyncConnection {
    /// Returns a tuple containing the inner connection and the tokio runtime the connection is running in.
    pub fn get_inner(&self) -> (PgPool, &tokio::runtime::Runtime) {
        (self.conn.conn(), &self.tokio)
    }

    ///creates a new connection to the database using the provided credentials and database name. this connection uses ssl mode prefer.
    pub fn new(
        user_name: &str,
        password: &str,
        host: &str,
        database: &str,
    ) -> Result<Self, DataBaseError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| DataBaseError::TokioError(e.to_string()))?;
        let conn = runtime.block_on(AsyncConnection::new(user_name, password, host, database))?;
        Ok(Self {
            conn,
            tokio: runtime,
        })
    }

    /// Creates a new connection from an existing pool and the tokio runtime it was created with and is running on.
    pub fn from_pool(pool: PgPool, linked_runtime: Runtime) -> Result<Self, DataBaseError> {
        let conn = AsyncConnection::from_pool(pool);
        Ok(Self {
            conn,
            tokio: linked_runtime,
        })
    }
    /// Creates a new connection to the database using the provided credentials, host, port, and database name. This connection uses ssl mode prefer.
    pub fn from_port(
        user_name: &str,
        password: &str,
        host: &str,
        port: usize,
        database: &str,
    ) -> Result<Self, DataBaseError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| DataBaseError::TokioError(e.to_string()))?;
        let conn = runtime.block_on(AsyncConnection::from_port(
            user_name, password, host, port, database,
        ))?;
        Ok(Self {
            conn,
            tokio: runtime,
        })
    }

    /// writes data to the database from a struct that implements the SQLformat trait.
    ///
    /// # Arguments
    /// * `data` - A vector of data to write to the database.
    /// * `table_name` - The name of the table to write to.
    /// * `columns` - A string containing the columns to write to, separated by commas.
    ///
    /// # Example
    /// ``` no_run
    /// use library::data_base_manager::*;
    ///
    ///     struct Test{
    ///     id:i64, value1:String, value2: bool
    ///     }
    ///     impl SQLformat<'_> for Test{
    ///      fn sqlformat(&self) -> Vec<ToSql>{
    ///         vec![ToSql::Bool(self.value2), ToSql::i64(self.id), ToSql::Text(&self.value1)]
    ///         }
    ///     }
    ///     let data = vec![Test{id:1,value1: "hello".to_string(), value2: true},
    ///                     Test{id:2,value1: "world".to_string(), value2: false},
    ///                     Test{id:5,value1: "cake".to_string(), value2: true}];
    ///     let mut conn = SyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").unwrap();
    ///
    ///     conn.write_database(data ,"test", "value2, id, value1");
    /// ```
    pub fn write_database<T>(
        &mut self,
        data: Vec<T>,
        table_name: &str,
        columns: &str,
    ) -> Result<(), DataBaseError>
    where
        T: for<'a> SQLformat<'a>,
    {
        self.tokio
            .block_on(self.conn.write_database(data, table_name, columns))
    }

    /// writes data to the database from a struct that implements the SQLformat trait, returning the number of rows written.
    ///
    /// # Arguments
    /// * `data` - A vector of data to write to the database.
    /// * `table_name` - The name of the table to write to.
    /// * `columns` - A string containing the columns to write to, separated by commas.
    ///
    /// # Example
    /// ``` no_run
    ///  use library::data_base_manager::*;
    ///
    ///     struct Test{
    ///     id:i64, value1:String, value2: bool
    ///     }
    ///     impl SQLformat<'_> for Test{
    ///      fn sqlformat(&self) -> Vec<ToSql>{
    ///         vec![ToSql::Bool(self.value2), ToSql::i64(self.id), ToSql::Text(&self.value1)]
    ///         }
    ///     }
    ///     let data = vec![Test{id:1,value1: "hello".to_string(), value2: true},
    ///                     Test{id:2,value1: "world".to_string(), value2: false},
    ///                     Test{id:5,value1: "cake".to_string(), value2: true}];
    ///     let mut conn = SyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").unwrap();
    ///
    ///     let count: u64 = conn.try_write_database(data ,"test", "value2, id, value1").unwrap();
    /// ```
    pub fn try_write_database<T>(
        &mut self,
        data: Vec<T>,
        table_name: &str,
        columns: &str,
    ) -> Result<u64, DataBaseError>
    where
        T: for<'a> SQLformat<'a>,
    {
        self.tokio
            .block_on(self.conn.try_write_database(data, table_name, columns))
    }

    /// Reads data from the database into a vector of structs that implement the SQLReadable trait.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to read from.
    /// * `columns` - A string containing the columns to read, separated by commas.
    /// * `condition` - A string containing the condition to filter the rows, e.g., "WHERE id > 10".
    ///
    /// # Example
    /// ``` no_run
    ///  
    ///     use library::data_base_manager::{*, sqlx::postgres::PgRow};
    ///     struct User {
    ///     id: i32, name: String, age: i32,
    ///     }
    ///
    /// impl SQLReadable for User {
    ///     fn from_row(row: &PgRow) -> Result<Self,DataBaseError> {
    ///         let id = get_colum_value(&row, "id")?;
    ///         let name = get_colum_value(&row, "name")?;
    ///         let age = get_colum_value(&row, "age")?;
    ///         Ok(User {
    ///             id, name, age
    ///         })
    ///     }
    /// }
    ///
    ///     let mut conn = SyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").unwrap();
    ///
    ///     let users: Vec<User> = conn.read_database("users", "id, name, age", "WHERE age > 18").unwrap();
    /// ```
    pub fn read_database<T>(
        &self,
        table_name: &str,
        columns: &str,
        condition: &str,
    ) -> Result<Vec<T>, DataBaseError>
    where
        T: SQLReadable,
    {
        self.tokio
            .block_on(self.conn.read_database(table_name, columns, condition))
    }

    /// Deletes entries from the database based on a condition.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to delete from.
    /// * `condition` - A string containing the condition to filter the rows to delete, e.g., "WHERE id = 10".
    ///
    /// # Example
    /// ``` no_run
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.delete_entry("users", "id = 1").unwrap();
    /// ```
    pub fn delete_entry(&self, table_name: &str, condition: &str) -> Result<u64, DataBaseError> {
        self.tokio
            .block_on(self.conn.delete_entry(table_name, condition))
    }

    /// Ensures a SQLite table meets the required format, creating or altering it as necessary.
    ///
    /// # Arguments
    /// * `table_name` - The name of the table to ensure the format for.
    /// * `columns` - A vector of `ColumnDefinition` that defines the columns and their properties.
    ///
    /// # Example
    /// ``` no_run
    ///
    ///  use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.ensure_table_format("users", &vec![
    ///     ColumnDefinition::new("id".to_string(), PostgresType::i64Auto, true ,true, true, None), // Primary key
    ///     ColumnDefinition::new("name".to_string(), PostgresType::String,true ,false, false, None),
    ///     ColumnDefinition::new("email".to_string(), PostgresType::String,true ,false, true, None)
    /// ]);
    ///
    /// ```
    pub fn ensure_table_format(
        &self,
        table_name: &str,
        required_columns: &[ColumnDefinition],
    ) -> Result<Option<String>, (Option<String>, DataBaseError)> {
        self.tokio
            .block_on(self.conn.ensure_table_format(table_name, required_columns))
    }

    /// Creates a new table in the `public` schema with the specified columns.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to create.
    /// - `required_columns`: A slice of `ColumnDefinition` specifying the columns and their constraints.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    /// conn.create_table("logs", &[
    ///     ColumnDefinition::new("id".to_string(), PostgresType::i64Auto, true, true, true, None),
    ///     ColumnDefinition::new("message".to_string(), PostgresType::String, true, false, false, None),
    /// ]).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL execution fails.
    pub fn create_table(
        &self,
        table_name: &str,
        required_columns: &[ColumnDefinition],
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.create_table(table_name, required_columns))
    }

    /// Drops the specified table from the `public` schema if it exists.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to drop.
    ///
    /// # Example
    /// ```no_run
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    /// conn.drop_table("logs").unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL execution fails.
    pub fn drop_table(&self, table_name: &str) -> Result<(), DataBaseError> {
        self.tokio.block_on(self.conn.drop_table(table_name))
    }

    /// Drops a column from the specified table in the `public` schema if it exists.
    ///
    /// # Arguments
    /// - `table_name`: The table containing the column.
    /// - `column_name`: The name of the column to drop.
    ///
    /// # Example
    /// ```no_run
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    /// conn.drop_column("users", "age").unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if either name is empty.
    /// Returns an error if the SQL execution fails.
    pub fn drop_column(&self, table_name: &str, column_name: &str) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.drop_column(table_name, column_name))
    }

    /// Checks if a table exists in the `public` schema.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     let exists = conn.table_exists("users").unwrap();
    /// ```
    ///
    /// # Returns
    /// `true` if the table exists, `false` otherwise.
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL query fails.
    pub fn table_exists(&self, table_name: &str) -> Result<bool, DataBaseError> {
        self.tokio.block_on(self.conn.table_exists(table_name))
    }

    /// Retrieves the current schema of a table from the `public` schema,
    /// including name, type, nullability, default, primary key, and uniqueness info.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    /// let schema:Vec<ColumnDefinition> = conn.get_table_scema("users").unwrap();
    /// ```
    ///
    /// # Returns
    /// A vector of `ColumnDefinition` representing the table’s schema.
    ///
    /// # Errors
    /// Returns an error if the SQL query fails or column values cannot be parsed.
    pub fn get_table_scema(
        &self,
        table_name: &str,
    ) -> Result<Vec<ColumnDefinition>, DataBaseError> {
        self.tokio.block_on(self.conn.get_table_scema(table_name))
    }

    /// Adds new columns to an existing table in the `public` schema.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to alter.
    /// - `column`: A slice of `ColumnDefinition` representing columns to add.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    /// conn.add_column_to_table("users", &[
    ///     ColumnDefinition::new("nickname".to_string(), PostgresType::String, false, false, false, None),
    /// ]).unwrap();
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if altering the table fails.
    pub fn add_column_to_table(
        &self,
        table_name: &str,
        column: &[ColumnDefinition],
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.add_column_to_table(table_name, column))
    }

    /// Grants permissions to a user on a specific table in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to grant permissions to.
    /// - `table`: The name of the table to grant permissions on.
    /// - `permissions`: A slice of `PgPermission` representing the permissions to grant.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.grant_permission("username", "my_table", &[PgPermission::Select, PgPermission::Insert]).unwrap();
    /// ```
    pub fn grant_permission(
        &self,
        user: &str,
        table: &str,
        permissions: &[PgPermission],
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.grant_permission(user, table, permissions))
    }

    /// Revokes permissions from a user on a specific table in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to revoke permissions from.
    /// - `table`: The name of the table to revoke permissions on.
    /// - `permissions`: A slice of `PgPermission` representing the permissions to revoke.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.revoke_permission("username", "my_table", &[PgPermission::Select, PgPermission::Insert]).unwrap();
    /// ```
    pub fn revoke_permission(
        &self,
        user: &str,
        table: &str,
        permissions: &[PgPermission],
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.revoke_permission(user, table, permissions))
    }

    /// Retrieves the permissions of a user on a specific table.
    ///
    /// # Arguments
    /// - `user`: The username to check permissions for.
    /// - `table`: The name of the table to check permissions on.
    ///
    /// # Returns
    /// - `UserPermissions`: A struct containing the table name, user name, and a vector of `PgPermission` representing the user's permissions on the table.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     let permissions = conn.get_permissions("username", "my_table").unwrap();
    /// ```
    pub fn get_permissions(
        &self,
        user: &str,
        table: &str,
    ) -> Result<UserPermissions, DataBaseError> {
        self.tokio.block_on(self.conn.get_permissions(user, table))
    }

    /// Adds a user to a group in the PostgreSQL database.
    ///
    /// # Arguments
    /// - `user`: The username to add to the group.
    /// - `group`: The name of the group to which the user will be added.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.add_user_to_group("username", "my_group").unwrap();
    /// ```
    pub fn add_user_to_group(
        &self,
        group_name: &str,
        user_name: &str,
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.add_user_to_group(group_name, user_name))
    }

    /// Removes a user from a group in the PostgreSQL database.
    ///
    /// # Arguments
    /// - `user`: The username to remove from the group.
    /// - `group`: The name of the group from which the user will be removed.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.remove_user_from_group("username", "my_group").unwrap();
    /// ```
    pub fn remove_user_from_group(
        &self,
        group_name: &str,
        user_name: &str,
    ) -> Result<(), DataBaseError> {
        self.tokio
            .block_on(self.conn.remove_user_from_group(group_name, user_name))
    }

    /// Grants permissions to a user on a specific sequence in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to grant permissions to.
    /// - `sequence`: The name of the table to grant permissions on.
    /// - `permissions`: A slice of `PgSequencePermission` representing the permissions to grant.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.grant_sequence_premission("username", "my_table", &[PgSequencePermission::Usage, PgSequencePermission::Select]).unwrap();
    /// ```
    pub fn grant_sequence_premission(
        &self,
        user: &str,
        sequence: &str,
        permissions: &[PgSequencePermission],
    ) -> Result<(), DataBaseError> {
        self.tokio.block_on(
            self.conn
                .grant_sequence_premission(user, sequence, permissions),
        )
    }

    /// Revokes permissions from a user on a specific sequence in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to revoke permissions from.
    /// - `sequence`: The name of the table to revoke permissions on.
    /// - `permissions`: A slice of `PgSequencePermission` representing the permissions to revoke.
    ///
    /// # Example
    /// ```no_run
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = SyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").unwrap();
    ///     conn.revoke_sequence_premission("username", "my_table", &[PgSequencePermission::Usage, PgSequencePermission::Select]).unwrap();
    /// ```
    pub fn revoke_sequence_premission(
        &self,
        user: &str,
        sequence: &str,
        permissions: &[PgSequencePermission],
    ) -> Result<(), DataBaseError> {
        self.tokio.block_on(
            self.conn
                .revoke_sequence_premission(user, sequence, permissions),
        )
    }
}

pub fn get_colum_value<T: for<'r> Decode<'r, Postgres> + Type<Postgres>>(
    row: &PgRow,
    colum_name: &str,
) -> Result<T, Error> {
    row.try_get::<T, &str>(&normalize_identifier(colum_name))
}

fn normalize_identifier(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') {
        s.to_string() // keep quoted identifiers unchanged
    } else {
        s.to_lowercase()
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use super::*;
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};

    fn get_runtime_and_pool() -> Result<(tokio::runtime::Runtime, PgPool), DataBaseError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| DataBaseError::TokioError(e.to_string()))?;

        let pool = rt.block_on(
            PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(Duration::from_secs(10))
                .connect("postgres://devtest:test@pi-server.home/devtest?sslmode=prefer"),
        )?;
        Ok((rt, pool))
    }

    #[derive(Debug)]
    struct TestItem {
        id: i32,
        value1: String,
        value2: bool,
    }

    impl SQLformat<'_> for TestItem {
        fn sqlformat(&self) -> Vec<ToSql> {
            vec![
                ToSql::Bool(self.value2),
                ToSql::i32(self.id),
                ToSql::Text(&self.value1),
            ]
        }
    }

    impl SQLReadable for TestItem {
        fn from_row(row: &PgRow) -> Result<Self, DataBaseError> {
            Ok(TestItem {
                id: row.get::<i32, usize>(0),
                value1: row.get::<_, usize>(1),
                value2: get_colum_value(row, "value2")?,
            })
        }
    }

    #[test]
    fn test_write_database_plain_insert() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_table_plain_insert";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (value2 BOOLEAN, id INTEGER PRIMARY KEY, value1 TEXT);"
            ))
            .execute(&pool),
        )?;

        // Ensure table exists
        let mut conn = SyncConnection::from_pool(pool.clone(), rt)?;

        let data = vec![
            TestItem {
                id: 10,
                value1: "Test A".to_string(),
                value2: true,
            },
            TestItem {
                id: 11,
                value1: "Test B".to_string(),
                value2: false,
            },
        ];

        conn.write_database(data, table, "value2, id, value1")?;
        let (_, rt) = conn.get_inner();

        let rows = rt.block_on(
            sqlx::query(&format!(
                "SELECT id, value1, value2 FROM {table} ORDER BY id"
            ))
            .fetch_all(&pool),
        )?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get::<i32, _>(0), 10);
        assert_eq!(rows[0].get::<String, _>(1), "Test A");
        assert_eq!(rows[0].get::<bool, _>(2), true);
        assert_eq!(rows[1].get::<i32, _>(0), 11);
        assert_eq!(rows[1].get::<String, _>(1), "Test B");
        assert_eq!(rows[1].get::<bool, _>(2), false);

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;

        Ok(())
    }

    #[test]
    fn test_write_database_empty_input() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_table_write_empty";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (value2 BOOLEAN, id INTEGER PRIMARY KEY, value1 TEXT);"
            ))
            .execute(&pool),
        )?;

        let mut conn = SyncConnection::from_pool(pool.clone(), rt)?;
        let data: Vec<TestItem> = vec![];
        conn.write_database(data, table, "value2, id, value1")?;

        let (_, rt) = conn.get_inner();

        let count: i64 = rt.block_on(
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}")).fetch_one(&pool),
        )?;
        assert_eq!(count, 0);

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_write_database_duplicate_primary_key() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_table_duplicates";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);"
            ))
            .execute(&pool),
        )?;

        let data = vec![
            TestItem {
                id: 1,
                value1: "First".to_string(),
                value2: true,
            },
            TestItem {
                id: 1,
                value1: "Duplicate".to_string(),
                value2: false,
            },
        ];

        let mut conn = SyncConnection::from_pool(pool.clone(), rt)?;
        let result = conn.write_database(data, table, "value2, id, value1");
        assert!(result.is_err());

        let (_, rt) = conn.get_inner();

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_read_database_invalid_columns() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool().unwrap();
        let table = "test_table_invalid_column";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))
            .unwrap();
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);"
            ))
            .execute(&pool),
        )
        .unwrap();

        let conn = SyncConnection::from_pool(pool.clone(), rt).unwrap();

        let result: Result<Vec<TestItem>, DataBaseError> =
            conn.read_database(table, "nonexistent_column", "");
        assert!(result.is_err());

        let (_, rt) = conn.get_inner();

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_read_database() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_table_read";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);"
            ))
            .execute(&pool),
        )?;

        rt.block_on(sqlx::query(&format!(
            "INSERT INTO {table} (id, value1, value2) VALUES (1, 'Hello', true), (2, 'World', false);"
        )).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;
        let rows: Vec<TestItem> =
            conn.read_database(table, "id, value1, value2", "WHERE value2 = true")?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[0].value1, "Hello");
        assert_eq!(rows[0].value2, true);

        let (_, rt) = conn.get_inner();

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_delete_entry() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_table_delete";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(
            sqlx::query(&format!(
                "CREATE TABLE {table} (id INTEGER PRIMARY KEY, value1 TEXT, value2 BOOLEAN);"
            ))
            .execute(&pool),
        )?;

        rt.block_on(sqlx::query(&format!(
            "INSERT INTO {table} (id, value1, value2) VALUES (1, 'Hello', true), (2, 'World', false);"
        )).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;
        let deleted_rows = conn.delete_entry(table, "id = 1")?;
        assert_eq!(deleted_rows, 1);

        let (_, rt) = conn.get_inner();

        let result = rt.block_on(
            sqlx::query(&format!("SELECT id, value1, value2 FROM {table}")).fetch_all(&pool),
        )?;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get::<i32, _>(0), 2);
        assert_eq!(result[0].get::<String, _>(1), "World");
        assert_eq!(result[0].get::<bool, _>(2), false);
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_create_table_with_ensure_table_format() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_create_table_with_ensure";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;

        conn.ensure_table_format(
            table,
            &vec![
                ColumnDefinition::new("id".to_string(), PostgresType::i32, true, true, false, None),
                ColumnDefinition::new(
                    "value1".to_string(),
                    PostgresType::String,
                    false,
                    false,
                    false,
                    None,
                ),
                ColumnDefinition::new(
                    "value2".to_string(),
                    PostgresType::bool,
                    false,
                    false,
                    false,
                    None,
                ),
            ],
        )
        .unwrap();

        let (_, rt) = conn.get_inner();

        // Check columns exist
        let rows = rt.block_on(
            sqlx::query(&format!(
                "SELECT column_name FROM information_schema.columns WHERE table_name = '{table}';"
            ))
            .fetch_all(&pool),
        )?;

        let column_names: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

        assert!(column_names.contains(&"id".to_string()));
        assert!(column_names.contains(&"value1".to_string()));
        assert!(column_names.contains(&"value2".to_string()));
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_extend_existing_table_with_new_columns() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_extend_table";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(sqlx::query(&format!("CREATE TABLE {table} (id INTEGER);")).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;

        conn.ensure_table_format(table, &vec![
                ColumnDefinition::new(
                    "id".to_string(),
                    PostgresType::i32,
                    false,
                    false,
                    false,
                    None,
                )]).unwrap();

        // Add additional optional columns
        conn.ensure_table_format(
            table,
            &vec![
                ColumnDefinition::new(
                    "id".to_string(),
                    PostgresType::i32,
                    false,
                    false,
                    false,
                    None,
                ),
                ColumnDefinition::new(
                    "value1".to_string(),
                    PostgresType::String,
                    false,
                    false,
                    false,
                    None,
                ),
                ColumnDefinition::new(
                    "value2".to_string(),
                    PostgresType::bool,
                    false,
                    false,
                    false,
                    None,
                ),
            ],
        )
        .unwrap();

        let (_, rt) = conn.get_inner();

        let rows = rt.block_on(
            sqlx::query(&format!(
                "SELECT column_name FROM information_schema.columns WHERE table_name = '{table}';"
            ))
            .fetch_all(&pool),
        )?;
        let column_names: Vec<String> = rows.iter().map(|r| r.get(0)).collect();

        assert!(column_names.contains(&"value1".to_string()));
        assert!(column_names.contains(&"value2".to_string()));
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_ensure_table_format_fails_on_invalid_constraint_change() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_constraint_conflict";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(sqlx::query(&format!("CREATE TABLE {table} (id INTEGER);")).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;

        // Try to redefine `id` as NOT NULL and PRIMARY KEY (should fail)
        let result = conn.ensure_table_format(
            table,
            &vec![ColumnDefinition::new(
                "id".to_string(),
                PostgresType::i32,
                true,
                true,
                false,
                None,
            )],
        );
        let (_, rt) = conn.get_inner();

        assert!(
        result.is_err(),
        "Expected ensure_table_format to fail when trying to add NOT NULL or PRIMARY KEY to existing column"
    );
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }

    #[test]
    fn test_table_exists() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_check_if_table_exists";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        rt.block_on(sqlx::query(&format!("CREATE TABLE {table} (id INTEGER);")).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;

        let x = conn.table_exists(table)?;

        assert!(x);
        let (pool, rt) = conn.get_inner();
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;

        let x = conn.table_exists(table)?;
        assert!(!x);
        Ok(())
    }

      #[test]
    fn test_get_colum_def() -> Result<(), DataBaseError> {
        let (rt, pool) = get_runtime_and_pool()?;
        let table = "test_get_column_def";

        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;

        let conn = SyncConnection::from_pool(pool.clone(), rt)?;

        let colums = vec![ColumnDefinition::new("id".to_string(),PostgresType::i16,false, true, false, None),
        ColumnDefinition::new("name".to_string(), PostgresType::String, false,false,false,None)]; 


        conn.ensure_table_format(table, &colums).unwrap();
        conn.ensure_table_format(table, &colums).unwrap();
        
        let x =conn.get_table_scema(table)?;

        println!("{:?}", colums);
        println!("{:?}", x);

        let (pool, rt) = conn.get_inner();
        rt.block_on(sqlx::query(&format!("DROP TABLE IF EXISTS {table}")).execute(&pool))?;
        Ok(())
    }
}
