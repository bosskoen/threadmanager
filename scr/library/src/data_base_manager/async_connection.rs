use std::collections::HashMap;

use sqlx::{postgres::PgQueryResult, PgPool};

use crate::data_base_manager::{
    can_add_column_to_non_empty_table, get_colum_value, normalize_identifier, ColumnDefinition,
    DataBaseError, PgPermission, PgSequencePermission, PostgresType, SQLReadable, SQLformat, ToSql,
    UserPermissions,
};

//TODO add support for non public schemas
pub struct AsyncConnection {
    conn: PgPool,
}
impl AsyncConnection {
    pub fn conn(&self) -> PgPool {
        self.conn.clone()
    }

    /// Creates a new `Connection` to a PostgreSQL database.
    /// # Example
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    /// use library::data_base_manager::*;
    /// let conn = AsyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").await.unwrap();
    ///
    /// });
    /// ```
    ///
    pub async fn new(
        user_name: &str,
        password: &str,
        host: &str,
        database: &str,
    ) -> Result<Self, DataBaseError> {
        AsyncConnection::from_port(user_name, password, host, 5432, database).await
    }

    pub fn from_pool(conn: PgPool) -> Self {
        Self { conn }
    }

    /// Creates a new `Connection` to a PostgreSQL database using a specific port.
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    /// use library::data_base_manager::*;
    /// let conn = AsyncConnection::from_port("myuser", "securepassword", "192.168.2.18", 5236, "mydatabase").await.unwrap();
    /// });
    /// ```
    pub async fn from_port(
        user_name: &str,
        password: &str,
        host: &str,
        port: usize,
        database: &str,
    ) -> Result<Self, DataBaseError> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=prefer",
            normalize_identifier(user_name),
            password,
            host,
            port,
            normalize_identifier(database)
        );
        let conn = PgPool::connect(&url).await?;
        Ok(Self { conn })
    }

    /// a simpel fucntion to write to a SQLite database.
    /// this function doesn't check if the table or colums esits
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check or create.
    /// - `data` : a vector that implements SQLformat.
    /// - `table_format` : a coma seperated string of colum names to tell this function were an how to write
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
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
    ///     let mut conn = AsyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").await.unwrap();
    ///
    ///     conn.write_database(&data ,"test", "value2, id, value1").await;
    ///
    /// });
    /// ```
    ///
    /// needs to move the data into the function, and data need to be of the same type
    pub async fn write_database<T>(
        &self,
        data: &[T],
        table_name: &str,
        table_format: &str,
    ) -> Result<(), DataBaseError>
    where
        T: for<'a> SQLformat<'a>,
    {
        if data.is_empty() {
            return Ok(());
        }
        let table_name = normalize_identifier(table_name);
        let table_format = normalize_identifier(table_format);

        let num_columns = table_format.split(',').count();

        let mut bound_args: Vec<ToSql<'_>> = Vec::with_capacity(data.len() * num_columns);

        data.iter().for_each(|item| {
            bound_args.extend(item.sqlformat());
        });

        let query_str = format!(
            "INSERT INTO public.{} ({}) VALUES {}",
            table_name,
            table_format,
            generate_placeholder(data.len(), num_columns)
        );

        let mut query = sqlx::query(&query_str);

        for arg in bound_args {
            query = match arg {
                ToSql::i32(val) => query.bind(val),
                ToSql::i64(val) => query.bind(val),
                ToSql::i16(val) => query.bind(val),
                ToSql::f32(val) => query.bind(val),
                ToSql::f64(val) => query.bind(val),
                ToSql::Text(val) => query.bind(val),
                ToSql::Bool(val) => query.bind(val),
            };
        }

        query.execute(&self.conn).await?;

        Ok(())
    }

    /// a simpel fucntion to write to a SQLite database.
    /// this function doesn't check if the table or colums esits
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check or create.
    /// - `data` : a vector that implements SQLformat.
    /// - `table_format` : a coma seperated string of colum names to tell this function were an how to write
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
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
    ///     let mut conn = AsyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").await.unwrap();
    ///
    ///     let count: u64 = conn.try_write_database(data ,"test", "value2, id, value1").await.unwrap();
    /// });
    /// ```
    ///
    pub async fn try_write_database<T>(
        &self,
        data: Vec<T>,
        table_name: &str,
        table_format: &str,
    ) -> Result<u64, DataBaseError>
    where
        T: for<'a> SQLformat<'a>,
    {
        if data.is_empty() {
            return Ok(0);
        }
        let table_name = normalize_identifier(table_name);
        let table_format = normalize_identifier(table_format);

        let num_columns = table_format.split(',').count();

        let command: String = format!(
            "INSERT INTO public.{} ({}) VALUES {} ON CONFLICT DO NOTHING",
            table_name,
            table_format,
            generate_placeholder(data.len(), num_columns)
        );

        let mut bound_args = Vec::with_capacity(data.len() * num_columns);
        data.iter().for_each(|item| {
            bound_args.extend(item.sqlformat());
        });

        let mut query: sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> =
            sqlx::query(&command);

        for arg in bound_args {
            query = match arg {
                ToSql::i32(val) => query.bind(val),
                ToSql::i64(val) => query.bind(val),
                ToSql::i16(val) => query.bind(val),
                ToSql::f32(val) => query.bind(val),
                ToSql::f64(val) => query.bind(val),
                ToSql::Text(val) => query.bind(val),
                ToSql::Bool(val) => query.bind(val),
            }
        }

        let result: PgQueryResult = query.execute(&self.conn).await?;
        Ok(result.rows_affected())
    }

    /// a simpel function to read a SQLite database
    /// this fuction doesn't check if your qerry is valid
    ///
    /// # Arguments
    ///
    /// - `table_name`: The name of the database table to query.
    /// - `query_column_names`: A comma-separated string of column names to select.
    /// - `condition`: A string representing the condition for the SQL query,
    ///   such as `"WHERE id = 1"`. Use an empty string if no condition is needed. with names are lower case
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
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
    ///     let mut conn = AsyncConnection::new("myuser", "securepassword", "localhost", "mydatabase").await.unwrap();
    ///
    ///     let users: Vec<User> = conn.read_database("users", "id, name, age", "WHERE age > 18").await.unwrap();
    /// });
    /// ```
    pub async fn read_database<T>(
        &self,
        table_name: &str,
        query_column_names: &str,
        condition: &str,
    ) -> Result<Vec<T>, DataBaseError>
    where
        T: SQLReadable,
    {
        let query_column_names = normalize_identifier(query_column_names);
        let table_name = normalize_identifier(table_name);
        let command = format!(
            "SELECT {} FROM public.{} {};",
            query_column_names, table_name, condition
        );

        let rows = sqlx::query(&command).fetch_all(&self.conn).await?;

        let mut x = Vec::with_capacity(rows.len());

        for row in rows {
            let item = T::from_row(&row)?;
            x.push(item);
        }
        Ok(x)
    }

    ///  Deletes an entry from the specified table based on the given condition.
    ///  give a error if condition is emptie
    ///
    /// # Arguments
    ///
    /// - `table_name`: The name of the table to delete from.
    /// - `condition`: The SQL condition specifying which rows to delete (e.g., `"id = 1"`). with names are lower case
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.delete_entry("users", "id = 1").await.unwrap();
    /// });
    /// ```
    pub async fn delete_entry(
        &self,
        table_name: &str,
        condition: &str,
    ) -> Result<u64, DataBaseError> {
        if condition.trim().is_empty() {
            return Err(DataBaseError::NoConditionFound(
                "Can't delete entire database at once".to_string(),
            )); // discriptor
        }

        let table_name = normalize_identifier(table_name);

        let command = format!("DELETE FROM public.{} WHERE {}", table_name, condition);
        Ok(sqlx::query(&command)
            .execute(&self.conn)
            .await?
            .rows_affected())
    }

    /// Ensures a Postgres table meets the required format, creating or altering it as necessary.
    /// this function only works with the public schema
    /// this function only adds columns, it does not remove them or change existing ones.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check or create.
    /// - `required_columns`: A vector of `ColumnDefinition` representing the required columns and their properties.
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.ensure_table_format("users", &vec![
    ///     ColumnDefinition::new("id".to_string(), PostgresType::i64Auto, true ,true, true, None), // Primary key
    ///     ColumnDefinition::new("name".to_string(), PostgresType::String ,false ,false, false, None),
    ///     ColumnDefinition::new("email".to_string(), PostgresType::String,true ,false, true, None),
    ///     ColumnDefinition::new("role".to_string(), PostgresType::String, true, false, false, Some("user".to_string())),
    /// ]).await;
    /// });
    /// ```
    pub async fn ensure_table_format(
        &self,
        table_name: &str,
        required_columns: &[ColumnDefinition],
    ) -> Result<Option<String>, (Option<String>, DataBaseError)> {
        let table_name = &normalize_identifier(table_name);

        // check if table exists
        let table_exists: bool = self
            .table_exists(table_name)
            .await
            .map_err(|e| (None, e.into()))?;

        // if not create it and return
        if !table_exists {
            self.create_table(table_name, &required_columns)
                .await
                .map_err(|e| (None, e.into()))?;
            return Ok(None);
        }

        // get table column metadata ( name | type | null | defealt ) ( PK and unique) and check for autincrement
        let existing_columns = self
            .get_table_scema(table_name)
            .await
            .map_err(|e| (None, e.into()))?;

        let existing_map: HashMap<String, ColumnDefinition> = existing_columns
            .into_iter()
            .map(|col| (col.name().to_string(), col))
            .collect();

        let mut unexpected_columns = Vec::new();
        let mut mismatched_columns = Vec::new();
        let mut missing_columns = Vec::new();

        // Check required columns
        for required in required_columns {
            match existing_map.get(required.name()) {
                Some(existing) => {
                    if !required.are_same(existing) {
                        mismatched_columns.push((
                            required.name().to_string(),
                            format!("{:?}", required),
                            format!("{:?}", existing),
                        ));
                    }
                }
                None => {
                    missing_columns.push(required.clone());
                }
            }
        }

        //Check for unexpected columns (present in DB but not in required schema)
        let required_names: HashMap<_, _> =
            required_columns.iter().map(|r| (r.name(), true)).collect();

        for existing_name in existing_map.keys() {
            if !required_names.contains_key(existing_name) {
                unexpected_columns.push(existing_name.clone());
            }
        }

        // Handle mismatches
        if !mismatched_columns.is_empty() {
            let mismatch_str = mismatched_columns
                .iter()
                .map(|(name, expected, found)| {
                    format!(
                        "Column `{}` mismatch\n  Expected: {}\n  Found:    {}",
                        name, expected, found
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let warning = if unexpected_columns.is_empty() {
                None
            } else {
                Some(format!(
                    "Unexpected columns: {}",
                    unexpected_columns.join(", ")
                ))
            };

            return Err((warning, DataBaseError::SchemaMismatchDetails(mismatch_str)));
        }

        // Add missing columns
        if !missing_columns.is_empty() {
            let warning = if unexpected_columns.is_empty() {
                None
            } else {
                Some(format!(
                    "Unexpected columns: {}",
                    unexpected_columns.join(", ")
                ))
            };

            self.add_column_to_table(table_name, &missing_columns)
                .await
                .map_err(|e| (warning.clone(), e.into()))?;

            return Ok(warning);
        }

        // No mismatches, nothing to add
        let warning = if unexpected_columns.is_empty() {
            None
        } else {
            Some(format!(
                "Unexpected columns: {}",
                unexpected_columns.join(", ")
            ))
        };

        Ok(warning)
    }

    //TODO next val en beginning of deafald

    /// Creates a new table in the `public` schema with the specified columns.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to create.
    /// - `required_columns`: A slice of `ColumnDefinition` specifying the columns and their constraints.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// conn.create_table("logs", &[
    ///     ColumnDefinition::new("id".to_string(), PostgresType::i64Auto, true, true, true, None),
    ///     ColumnDefinition::new("message".to_string(), PostgresType::String, true, false, false, None),
    /// ]).await.unwrap();
    /// });
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL execution fails.
    pub async fn create_table(
        &self,
        table_name: &str,
        required_columns: &[ColumnDefinition],
    ) -> Result<(), DataBaseError> {
        if table_name.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "Table name cannot be empty".to_string(),
            ));
        }
        let table_name = normalize_identifier(table_name);
        let primary_keys: Vec<&str> = required_columns
            .iter()
            .filter(|col| col.primary_key())
            .map(|col| col.name().as_str())
            .collect();

        let is_single_pk = primary_keys.len() == 1;

        let mut column_defs: Vec<String> = required_columns
            .iter()
            .map(|col| col.to_sql(is_single_pk))
            .collect();

        if primary_keys.len() > 1 {
            let pk_clause = format!("PRIMARY KEY ({})", primary_keys.join(", "));
            column_defs.push(pk_clause);
        }

        let columns_query = column_defs.join(",\n\t");
        let create_table_query = format!(
            "CREATE TABLE public.{} (\n\t{}\n);",
            table_name, columns_query
        );

        sqlx::query(&create_table_query).execute(&self.conn).await?;

        Ok(())
    }

    /// Drops the specified table from the `public` schema if it exists.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to drop.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// conn.drop_table("logs").await.unwrap();
    /// });
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL execution fails.
    pub async fn drop_table(&self, table_name: &str) -> Result<(), DataBaseError> {
        if table_name.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "Table name cannot be empty".to_string(),
            ));
        }
        let table_name = normalize_identifier(table_name);
        let drop_table_query = format!("DROP TABLE IF EXISTS public.{};", table_name);
        sqlx::query(&drop_table_query).execute(&self.conn).await?;
        Ok(())
    }

    /// Drops a column from the specified table in the `public` schema if it exists.
    ///
    /// # Arguments
    /// - `table_name`: The table containing the column.
    /// - `column_name`: The name of the column to drop.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// conn.drop_column("users", "age").await.unwrap();
    /// });
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if either name is empty.
    /// Returns an error if the SQL execution fails.
    pub async fn drop_column(
        &self,
        table_name: &str,
        column_name: &str,
    ) -> Result<(), DataBaseError> {
        if table_name.is_empty() || column_name.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "Table name or column name cannot be empty".to_string(),
            ));
        }
        let table_name = normalize_identifier(table_name);
        let column_name = normalize_identifier(column_name);
        let drop_column_query = format!(
            "ALTER TABLE public.{} DROP COLUMN IF EXISTS {};",
            table_name, column_name
        );
        sqlx::query(&drop_column_query).execute(&self.conn).await?;
        Ok(())
    }

    /// Checks if a table exists in the `public` schema.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// let exists = conn.table_exists("users").await.unwrap();
    /// });
    /// ```
    ///
    /// # Returns
    /// `true` if the table exists, `false` otherwise.
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if the SQL query fails.
    pub async fn table_exists(&self, table_name: &str) -> Result<bool, DataBaseError> {
        if table_name.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "Table name cannot be empty".to_string(),
            ));
        }
                let table_name = normalize_identifier(table_name);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1)"
        )
        .bind(table_name)
        .fetch_one(&self.conn)
        .await?;
        Ok(exists)
    }

    /// Retrieves the current schema of a table from the `public` schema,
    /// including name, type, nullability, default, primary key, and uniqueness info.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// let schema:Vec<ColumnDefinition> = conn.get_table_scema("users").await.unwrap();
    /// });
    /// ```
    ///
    /// # Returns
    /// A vector of `ColumnDefinition` representing the table’s schema.
    ///
    /// # Errors
    /// Returns an error if the SQL query fails or column values cannot be parsed.
    pub async fn get_table_scema(
        &self,
        table_name: &str,
    ) -> Result<Vec<ColumnDefinition>, DataBaseError> {
        let table_name = normalize_identifier(table_name);
        let existing_columns = sqlx::query(
            r#"
    SELECT
        col.column_name,
        (col.is_nullable = 'NO')::bool as not_null,
        col.data_type,
        col.column_default,
        (ct.constraint_type = 'PRIMARY KEY')::bool as is_primary_key,
        (ct.constraint_type = 'UNIQUE')::bool as is_unique
    FROM information_schema.columns col
    LEFT JOIN information_schema.key_column_usage kcu
        ON col.table_name = kcu.table_name
        AND col.column_name = kcu.column_name
        AND col.table_schema = kcu.table_schema
    LEFT JOIN information_schema.table_constraints ct
        ON kcu.constraint_name = ct.constraint_name
        AND kcu.table_schema = ct.table_schema
    WHERE col.table_name = $1 AND col.table_schema = 'public'
    ORDER BY col.ordinal_position
    "#,
        )
        .bind(table_name)
        .fetch_all(&self.conn)
        .await?;

        let mut existing_columns_def: Vec<ColumnDefinition> =
            Vec::with_capacity(existing_columns.len());
        for row in existing_columns {
            let column_name: String = get_colum_value(&row, "column_name")?;
            let not_null: bool = get_colum_value(&row, "not_null")?;
            let data_type: String = get_colum_value(&row, "data_type")?;
            let column_default: Option<String> = get_colum_value(&row, "column_default")?;
            let is_primary_key: bool =
                get_colum_value::<Option<bool>>(&row, "is_primary_key")?.unwrap_or(false);
            let is_unique: bool =
                get_colum_value::<Option<bool>>(&row, "is_unique")?.unwrap_or(false);

            let col_type = PostgresType::from_sql_type(
                &data_type,
                column_default
                    .as_ref()
                    .map_or(false, |defalt| defalt.contains("nextval(")),
            )
            .unwrap_or(PostgresType::String);

            existing_columns_def.push(ColumnDefinition::new(
                column_name,
                col_type,
                not_null,
                is_primary_key,
                is_unique,
                column_default,
            ));
        }

        Ok(existing_columns_def)
    }

    /// Adds new columns to an existing table in the `public` schema.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to alter.
    /// - `column`: A slice of `ColumnDefinition` representing columns to add.
    ///
    /// # Example
    /// ```no_run
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::data_base_manager::*;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    /// conn.add_column_to_table("users", &[
    ///     ColumnDefinition::new("nickname".to_string(), PostgresType::String, false, false, false, None),
    /// ]).await.unwrap();
    /// });
    /// ```
    ///
    /// # Errors
    /// Returns `DataBaseError::InvalidCondition` if the table name is empty.
    /// Returns an error if altering the table fails.
    pub async fn add_column_to_table(
        &self,
        table_name: &str,
        column: &[ColumnDefinition],
    ) -> Result<(), DataBaseError> {
        if column.is_empty() {
            return Ok(()); // Nothing to do
        }

        let table_name = normalize_identifier(table_name);
        if table_name.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "Table name or column name cannot be empty".to_string(),
            ));
        }

        let is_empty: bool = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) = 0 AS is_empty FROM public.{};",
            table_name
        ))
        .fetch_one(&self.conn)
        .await?;

        if !is_empty {
            for x in column {
                if !can_add_column_to_non_empty_table(x) {
                    return Err(DataBaseError::SchemaMismatchDetails(format!(
                        "can add column: {:?}\nveledates table integetie",
                        x
                    )));
                }
            }
        }

        for missing in column {
            let alter_stmt = format!(
                "ALTER TABLE public.{} ADD COLUMN {}",
                table_name,
                missing.to_sql(true)
            );
            sqlx::query(&alter_stmt).execute(&self.conn).await?;
        }
        Ok(())
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
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.grant_permission("username", "my_table", &[PgPermission::Select, PgPermission::Insert]).await.unwrap();
    /// });
    /// ```
    pub async fn grant_permission(
        &self,
        user: &str,
        table: &str,
        permissions: &[PgPermission],
    ) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || table.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User, table, or permissions cannot be empty".to_string(),
            ));
        }

        let table = normalize_identifier(table);
        let user = normalize_identifier(user);

        let grant_all: bool = permissions.contains(&PgPermission::All);
        let command;
        if grant_all {
            command = format!(
                "GRANT ALL PRIVILEGES ON TABLE public.{} TO {};",
                table, user
            );
        } else {
            command = format!(
                "GRANT {} ON TABLE public.{} TO {};",
                permissions
                    .iter()
                    .map(|p| match p {
                        PgPermission::Select => "SELECT",
                        PgPermission::Insert => "INSERT",
                        PgPermission::Update => "UPDATE",
                        PgPermission::Delete => "DELETE",
                        PgPermission::All => "ALL PRIVILEGES",
                        PgPermission::Truncate => "TRUNCATE",
                        PgPermission::References => "REFERENCES",
                        PgPermission::Trigger => "TRIGGER",
                    })
                    .collect::<Vec<&str>>()
                    .join(", "),
                table,
                user
            );
        }
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
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
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.revoke_permission("username", "my_table", &[PgPermission::Select, PgPermission::Insert]).await.unwrap();
    /// });
    /// ```
    pub async fn revoke_permission(
        &self,
        user: &str,
        table: &str,
        permissions: &[PgPermission],
    ) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || table.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User, table, or permissions cannot be empty".to_string(),
            ));
        }
        let table = normalize_identifier(table);
        let user = normalize_identifier(user);
        let revokeall: bool = permissions.contains(&PgPermission::All);

        let command;
        if revokeall {
            command = format!(
                "REVOKE ALL PRIVILEGES ON TABLE public.{} FROM {};",
                table, user
            );
        } else {
            command = format!(
                "REVOKE {} ON TABLE public.{} FROM {};",
                permissions
                    .iter()
                    .map(|p| match p {
                        PgPermission::Select => "SELECT",
                        PgPermission::Insert => "INSERT",
                        PgPermission::Update => "UPDATE",
                        PgPermission::Delete => "DELETE",
                        PgPermission::All => "ALL PRIVILEGES",
                        PgPermission::Truncate => "TRUNCATE",
                        PgPermission::References => "REFERENCES",
                        PgPermission::Trigger => "TRIGGER",
                    })
                    .collect::<Vec<&str>>()
                    .join(", "),
                table,
                user
            );
        }
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
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
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     let permissions = conn.get_permissions("username", "my_table").await.unwrap();
    /// });
    /// ```
    pub async fn get_permissions(
        &self,
        user: &str,
        table: &str,
    ) -> Result<UserPermissions, DataBaseError> {
        let table = &normalize_identifier(table);
        let user = &normalize_identifier(user);
        let row = sqlx::query(
            r#"
    SELECT privilege_type
    FROM information_schema.role_table_grants
    WHERE grantee = $1
      AND table_schema = 'public'
      AND table_name = $3
    "#,
        )
        .bind(user)
        .bind(table)
        .fetch_all(&self.conn)
        .await?;
        let mut permisiosn: Vec<PgPermission> = Vec::with_capacity(row.len());
        for r in row {
            let privilege_type: String = get_colum_value(&r, "privilege_type")?;
            let perm = match privilege_type.as_str() {
                "SELECT" => PgPermission::Select,
                "INSERT" => PgPermission::Insert,
                "UPDATE" => PgPermission::Update,
                "DELETE" => PgPermission::Delete,
                "TRUNCATE" => PgPermission::Truncate,
                "REFERENCES" => PgPermission::References,
                "TRIGGER" => PgPermission::Trigger,
                _ => {
                    return Err(DataBaseError::UnknownColumnType(format!(
                        "Unknown permission type: {}",
                        privilege_type
                    )))
                }
            };
            permisiosn.push(perm);
        }

        Ok(UserPermissions {
            table: table.to_string(),
            user: user.to_string(),
            permissions: permisiosn,
        })
    }

    /// Adds a user to a group in the PostgreSQL database.
    ///
    /// # Arguments
    /// - `user`: The username to add to the group.
    /// - `group`: The name of the group to which the user will be added.
    ///
    /// # Example
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.add_user_to_group("username", "my_group").await.unwrap();
    /// });
    /// ```
    pub async fn add_user_to_group(&self, user: &str, group: &str) -> Result<(), DataBaseError> {
        if user.is_empty() || group.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User or group cannot be empty".to_string(),
            ));
        }
        let user = normalize_identifier(user);
        let group = normalize_identifier(group);

        let command = format!("GRANT {} TO {};", group, user);
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
    }

    /// Removes a user from a group in the PostgreSQL database.
    ///
    /// # Arguments
    /// - `user`: The username to remove from the group.
    /// - `group`: The name of the group from which the user will be removed.
    ///
    /// # Example
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.remove_user_from_group("username", "my_group").await.unwrap();
    /// });
    /// ```
    pub async fn remove_user_from_group(
        &self,
        user: &str,
        group: &str,
    ) -> Result<(), DataBaseError> {
        if user.is_empty() || group.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User or group cannot be empty".to_string(),
            ));
        }
        let user = normalize_identifier(user);
        let group = normalize_identifier(group);
        let command = format!("REVOKE {} FROM {};", group, user);
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
    }

    //TODO update privalige into one function

    /// Grants permissions to a user on a specific sequence in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to grant permissions to.
    /// - `sequence`: The name of the sequence to grant permissions on.
    /// - `permissions`: A slice of `PgSequencePermission` representing the permissions to grant.
    ///
    /// # Example
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.grant_sequence_premission("username", "my_sequence", &[PgSequencePermission::Usage, PgSequencePermission::Select]).await.unwrap();
    /// });
    /// ```
    pub async fn grant_sequence_premission(
        &self,
        user: &str,
        sequence: &str,
        permissions: &[PgSequencePermission],
    ) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || sequence.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User, sequence, or permissions cannot be empty".to_string(),
            ));
        }
        let user = normalize_identifier(user);
        let sequence = normalize_identifier(sequence);

        let revokeall: bool = permissions.contains(&PgSequencePermission::All);

        let command;
        if revokeall {
            command = format!("GRANT ALL ON SEQUENCE public.{} TO {};", sequence, user);
        } else {
            command = format!(
                "GRANT {} ON SEQUENCE public.{} TO {};",
                permissions
                    .iter()
                    .map(|p| match p {
                        PgSequencePermission::Select => "SELECT",
                        PgSequencePermission::Update => "UPDATE",
                        PgSequencePermission::All => "ALL",
                        PgSequencePermission::Usage => "USAGE",
                    })
                    .collect::<Vec<&str>>()
                    .join(", "),
                sequence,
                user
            );
        }
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
    }

    /// Revokes permissions from a user on a specific sequence in the public schema.
    ///
    /// # Arguments
    /// - `user`: The username to grant permissions to.
    /// - `sequence`: The name of the sequence to grant permissions on.
    /// - `permissions`: A slice of `PgSequencePermission` representing the permissions to revoke.
    ///
    /// # Example
    /// ```no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    ///
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.revoke_sequence_premission("username", "my_sequence", &[PgSequencePermission::Usage, PgSequencePermission::Select]).await.unwrap();
    /// });
    /// ```
    pub async fn revoke_sequence_premission(
        &self,
        user: &str,
        sequence: &str,
        permissions: &[PgSequencePermission],
    ) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || sequence.is_empty() {
            return Err(DataBaseError::InvalidCondition(
                "User, sequence, or permissions cannot be empty".to_string(),
            ));
        }
        let user = normalize_identifier(user);
        let sequence = normalize_identifier(sequence);

        let revokeall: bool = permissions.contains(&PgSequencePermission::All);

        let command;
        if revokeall {
            command = format!("REVOKE ALL ON SEQUENCE public.{} FROM {};", sequence, user);
        } else {
            command = format!(
                "REVOKE {} ON SEQUENCE  public.{} FROM {};",
                permissions
                    .iter()
                    .map(|p| match p {
                        PgSequencePermission::Select => "SELECT",
                        PgSequencePermission::Update => "UPDATE",
                        PgSequencePermission::All => "ALL",
                        PgSequencePermission::Usage => "USAGE",
                    })
                    .collect::<Vec<&str>>()
                    .join(", "),
                sequence,
                user
            );
        }
        let _ = sqlx::query(&command).execute(&self.conn).await?;
        Ok(())
    }
}

fn generate_placeholder(data_len: usize, num_columns: usize) -> String {
    let mut placeholders = String::with_capacity(data_len * num_columns * 4);

    for i in 0..data_len {
        placeholders.push('(');
        for j in 0..num_columns {
            let index = i * num_columns + j + 1;
            use std::fmt::Write;
            write!(placeholders, "${}", index).unwrap();
            if j < num_columns - 1 {
                placeholders.push_str(", ");
            }
        }
        placeholders.push(')');
        if i < data_len - 1 {
            placeholders.push_str(", ");
        }
    }

    placeholders
}
