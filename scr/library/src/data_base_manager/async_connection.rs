use std::collections::HashSet;

use sqlx::{postgres::PgQueryResult, PgPool};

use crate::data_base_manager::{
    get_colum_value, helper_functions::{alter_table, create_table, generate_placeholder, InternalColumnDef}, ColumnDefinition, DataBaseError, PgPermission, PostgresType, SQLReadable, SQLformat, ToSql, UserPermissions
};

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
        let url = format!(
            "postgres://{}:{}@{}/{}?sslmode=prefer",
            user_name, password, host, database
        );
        let conn = PgPool::connect(&url).await?;
        Ok(Self { conn })
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
            user_name, password, host, port, database
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
    ///     conn.write_database(data ,"test", "value2, id, value1").await;
    /// 
    /// });
    /// ```
    ///
    /// needs to move the data into the function, and data need to be of the same type
    pub async fn write_database<T>(
        &self,
        data: Vec<T>,
        table_name: &str,
        table_format: &str,
    ) -> Result<(), DataBaseError>
    where
        T: for<'a> SQLformat<'a>,
    {
        if data.is_empty() {
            return Ok(());
        }
        let num_columns = table_format.split(',').count();

        let mut bound_args: Vec<ToSql<'_>> = Vec::with_capacity(data.len() * num_columns);

        data.iter().for_each(|item| {
            bound_args.extend(item.sqlformat());
        });

        let query_str = format!(
            "INSERT INTO {} ({}) VALUES {}",
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
        let num_columns = table_format.split(',').count();

        let command: String = format!(
            "INSERT INTO {} ({}) VALUES {} ON CONFLICT DO NOTHING",
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
    ///   such as `"WHERE id = 1"`. Use an empty string if no condition is needed.
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
        let command = format!(
            "SELECT {} FROM {} {};",
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
    /// - `condition`: The SQL condition specifying which rows to delete (e.g., `"id = 1"`).
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

        let command = format!("DELETE FROM {} WHERE {}", table_name, condition);
        Ok(sqlx::query(&command)
            .execute(&self.conn)
            .await?
            .rows_affected())
    }

    //TODO add defeald value.

    /// Ensures a SQLite table meets the required format, creating or altering it as necessary.
    ///
    /// # Arguments
    /// - `table_name`: Name of the table to check or create.
    /// - `required_columns`: A vector of tuples (column name, column type, not_null ,is_primary_key).
    ///
    /// # Example
    /// ``` no_run
    /// // asyncConnection needs to run in a tokio runtime.
    /// tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
    /// 
    ///     use library::{*,data_base_manager::*} ;
    ///     let conn = AsyncConnection::new("myuser", "securepassword", "192.168.2.18", "mydatabase").await.unwrap();
    ///     conn.ensure_table_format("users", vec![
    ///     define_column!("id", PostgresType::i64, true ,true), // Primary key
    ///     define_column!("name", PostgresType::String,false ,false),
    ///     define_column!("email", PostgresType::String,true ,false)
    /// ]).await;
    /// });
    /// ```
    pub async fn ensure_table_format(
        &self,
        table_name: &str,
        required_columns: Vec<ColumnDefinition<'_>>, //TODO slice maby
    ) -> Result<(), DataBaseError> {
        let mut lock_tracaction = self.conn.begin().await?;

        // 1. Query existing columns metadata
        let columns = sqlx::query(
            r#"
        SELECT
    column_name,
    data_type,
    is_nullable,
    column_default
FROM information_schema.columns
WHERE table_name = $1
ORDER BY ordinal_position
        "#,
        )
        .bind(table_name)
        .fetch_all(&mut *lock_tracaction)
        .await?;

        // 2. Query primary key columns
        let pk_rows = sqlx::query(
            r#"
        SELECT kcu.column_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
          AND tc.table_schema = kcu.table_schema
        WHERE tc.table_name = $1
          AND tc.constraint_type = 'PRIMARY KEY'
        "#,
        )
        .bind(table_name)
        .fetch_all(&mut *lock_tracaction)
        .await?;

        // 3. Build existing_columns Vec<ColumnDefinition>
        let mut pk_columns: std::collections::HashSet<_> = HashSet::new();

        for r in pk_rows {
            pk_columns.insert(get_colum_value::<String>(&r, "column_name")?);
        }

        let mut existing_columns: Vec<InternalColumnDef> = Vec::new();

        for row in columns {
            let name = get_colum_value(&row, "column_name")?;
            let is_pk = pk_columns.contains(&name);

            let col_type: String = get_colum_value(&row, "data_type")?;

            let col_default = get_colum_value::<Option<String>>(&row, "column_default")?
                .as_deref()
                .map_or(false, |d| d.starts_with("nextval("));

            let pg_type = PostgresType::from_sql_type(&col_type, col_default).ok_or(
                DataBaseError::UnknownColumnType(format!("Unknown PostgreSQL type: {}", col_type)),
            )?;

            let is_not_null = get_colum_value::<String>(&row, "is_nullable")? == "NO";

            existing_columns.push(InternalColumnDef::new(name, pg_type, is_not_null, is_pk));
        }

        if existing_columns.is_empty() {
            create_table(&mut lock_tracaction, &required_columns, table_name).await?;
        } else {
            alter_table(
                &mut lock_tracaction,
                &required_columns,
                &existing_columns,
                table_name,
            )
            .await?;
        }
        lock_tracaction.commit().await?;
        Ok(())
    }


    /// Grants permissions to a user on a specific table.
    /// If the `permissions` vector is empty, it grants all privileges.
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
    pub async fn grant_permission(&self, user: &str, table: &str, permissions: &[PgPermission]) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || table.is_empty() {
            return Err(DataBaseError::InvalidCondition("User, table, or permissions cannot be empty".to_string()));
        }
        let grant_all:bool = permissions.contains(&PgPermission::All);
        let command;
        if grant_all {
            command = format!("GRANT ALL PRIVILEGES ON TABLE {} TO {};",
                table, user);
        }else {
        command = format!(
            "GRANT {} ON TABLE {} TO {};",
            permissions.iter().map(|p| match p {
                PgPermission::Select=>"SELECT",
                PgPermission::Insert=>"INSERT",
                PgPermission::Update=>"UPDATE",
                PgPermission::Delete=>"DELETE",
                PgPermission::All=>"ALL PRIVILEGES",
                PgPermission::Truncate => "TRUNCATE",
                PgPermission::References => "REFERENCES",
                PgPermission::Trigger => "TRIGGER",
            }).collect::<Vec<&str>>().join(", "),
            table,
            user
        );
        }
        let _ = sqlx::query(&command)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    /// Revokes permissions from a user on a specific table.
    /// If the `permissions` vector is empty, it revokes all privileges.
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
    pub async fn revoke_permission(&self, user: &str, table: &str, permissions: &[PgPermission]) -> Result<(), DataBaseError> {
        if permissions.is_empty() || user.is_empty() || table.is_empty() {
            return Err(DataBaseError::InvalidCondition("User, table, or permissions cannot be empty".to_string()));
        }
        let revokeall:bool = permissions.contains(&PgPermission::All);

        let command;
        if revokeall {
            command = format!("REVOKE ALL PRIVILEGES ON TABLE {} FROM {};",
                table, user);
        }else {
            command = format!(
            "REVOKE {} ON TABLE {} FROM {};",
            permissions.iter().map(|p| match p {
                PgPermission::Select=>"SELECT",
                PgPermission::Insert=>"INSERT",
                PgPermission::Update=>"UPDATE",
                PgPermission::Delete=>"DELETE",
                PgPermission::All=>"ALL PRIVILEGES",
                PgPermission::Truncate => "TRUNCATE",
                PgPermission::References => "REFERENCES",
                PgPermission::Trigger => "TRIGGER",
                }).collect::<Vec<&str>>().join(", "),
            table,
            user
        );
        }
        let _ = sqlx::query(&command)
            .execute(&self.conn)
            .await?;
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
    pub async fn get_permissions(&self, user: &str, table: &str) -> Result<UserPermissions, DataBaseError> {
        let row = sqlx::query(r#"select privilege_type FROM information_schema.role_table_grants WHERE grantee = $1 AND table_name = $2"#)
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
                _ => return Err(DataBaseError::UnknownColumnType(format!("Unknown permission type: {}", privilege_type))),
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
            return Err(DataBaseError::InvalidCondition("User or group cannot be empty".to_string()));
        }
        let command = format!("GRANT {} TO {};", group, user);
        let _ = sqlx::query(&command)
            .execute(&self.conn)
            .await?;
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
    pub async fn remove_user_from_group(&self, user: &str, group: &str) -> Result<(), DataBaseError> {
        if user.is_empty() || group.is_empty() {
            return Err(DataBaseError::InvalidCondition("User or group cannot be empty".to_string()));
        }
        let command = format!("REVOKE {} FROM {};", group, user);
        let _ = sqlx::query(&command)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

}

