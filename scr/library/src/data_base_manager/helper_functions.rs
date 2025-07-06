use sqlx::Transaction;

use crate::data_base_manager::PostgresType;

use super::{ColumnDefinition, DataBaseError};

pub fn generate_placeholder(data_len: usize, num_columns: usize) -> String {
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

pub async fn create_table(
    conn: &mut Transaction<'_, sqlx::Postgres>,
    required_columns: &[ColumnDefinition<'_>],
    table_name: &str,
) -> Result<(), DataBaseError> {
    let mut primary_key = String::new();

    let columns_def = required_columns
        .iter()
        .map(|new_colum| {
            let mut query = format!("{} {}", new_colum.name(), new_colum.col_type().to_sql_type());
            if new_colum.not_null() {
                query.push_str(" NOT NULL");
            }
            if new_colum.is_primary_key() {
                if primary_key.len() == 0 {
                    primary_key.push_str(",\nPRIMARY KEY (");
                    primary_key.push_str(new_colum.name());
                } else {
                    primary_key.push(',');
                    primary_key.push_str(new_colum.name());
                }
            }
            query
        })
        .collect::<Vec<_>>()
        .join(", ");
    if primary_key.len() != 0 {
        primary_key.push(')');
    }
    let create_table_query = format!(
        "CREATE TABLE {} ({}{});",
        table_name, columns_def, primary_key
    );
    sqlx::query(&create_table_query)
        .execute(&mut **conn)
        .await?;
    Ok(())
}

pub struct InternalColumnDef {
    pub name: String,
    pub col_type: PostgresType,
    pub not_null: bool,
    pub is_primary_key: bool,
}

impl InternalColumnDef {
    pub fn new(name: String, col_type: PostgresType, not_null: bool, is_primary_key: bool) -> Self {
        InternalColumnDef {
            name,
            col_type,
            not_null,
            is_primary_key,
        }
    }
}

//TODO can add not null and serial seport
pub async fn alter_table(
    conn: &mut Transaction<'_, sqlx::Postgres>,
    required_columns: &[ColumnDefinition<'_>],
    existing_columns: &[InternalColumnDef],
    table_name: &str,
) -> Result<(), DataBaseError> {
    let mut querys: Vec<String> = Vec::new();
    let mut errors: Vec<String> = vec!["Errors while updating table fomat:\n".to_string()];
    let mut main_error_flag = false;
    for required_colum in required_columns {
        match existing_columns
            .iter()
            .find(|existing_colum| existing_colum.name == required_colum.name())
        {
            Some(existing_colum) => {
                if let Some(error_message) =
                    check_column_mismatch(existing_colum, required_colum, table_name)
                {
                    errors.push(error_message);
                    main_error_flag = true;
                }
            }
            None => {
                if required_colum.col_type() == PostgresType::i16Auto
                    || required_colum.col_type() == PostgresType::i32Auto
                    || required_colum.col_type() == PostgresType::i64Auto
                {
                    //TODO add serial support
                    errors.push(format!(
                        "Tryed to add serial collem: \"{}\" to existing table \"{}\"\n",
                        required_colum.name(),
                        table_name
                    ));
                    main_error_flag = true;
                }

                if required_colum.is_primary_key() {
                    main_error_flag = true;
                    errors.push(format!(
                        "Tryed to add primary key collem: \"{}\" to existing table \"{}\"\n",
                        required_colum.name(),
                        table_name
                    ));
                } else {
                    if required_colum.not_null() {
                        querys.push(format!(
                            "ALTER TABLE {} ADD COLUMN {} {} NOT NULL;", //TODO will trow error so needs default value
                            table_name,
                            required_colum.name(),
                            required_colum.col_type().to_sql_type()
                        ));
                    } else {
                        querys.push(format!(
                            "ALTER TABLE {} ADD COLUMN {} {};",
                            table_name,
                            required_colum.name(),
                            required_colum.col_type().to_sql_type()
                        ));
                    }
                }
            }
        }
    }
    if main_error_flag {
        return Err(DataBaseError::AlterTableError(errors.join("")));
    }

    for query in querys {
        sqlx::query(&query).execute(&mut **conn).await?;
    }
    Ok(())
}

fn check_column_mismatch(
    existing_colum: &InternalColumnDef,
    required_colum: &ColumnDefinition,
    table_name: &str,
) -> Option<String> {
    let mut error_flag = false;
    let mut error_mesige = String::new();

    if existing_colum.col_type != required_colum.col_type() {
        error_flag = true;
        error_mesige.push_str(&format!(
            "- Type mismatch: expected '{}', found '{}'.\n",
            required_colum.col_type().to_sql_type(),
            existing_colum.col_type.to_sql_type()
        ));
    }
    if existing_colum.not_null != required_colum.not_null() {
        error_flag = true;
        error_mesige.push_str(&format!(
            "- NOT NULL mismatch: expected '{}', found '{}'.\n",
            required_colum.not_null(),
            existing_colum.not_null
        ));
    }
    if existing_colum.is_primary_key != required_colum.is_primary_key() {
        error_flag = true;
        error_mesige.push_str(&format!(
            "- PRIMARY KEY mismatch: expected '{}', found '{}'.\n",
            required_colum.is_primary_key(),
            existing_colum.is_primary_key
        ));
    }
    if error_flag {
        error_mesige.insert_str(
            0,
            &format!(
                "Already existing column: \"{}\" in table: \"{}\" didn't match given format:\n",
                existing_colum.name, table_name
            ),
        );
        return Some(error_mesige);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_base_manager::{ColumnDefinition, DataBaseError};
    use sqlx::PgPool;
    use sqlx::Row;

    #[tokio::test]
    async fn test_alter_table_add_column() -> Result<(), DataBaseError> {
        let pool = PgPool::connect(
            "postgres://devtest:test@localhost/devtest?sslmode=prefer").await?;
        let mut tx = pool.begin().await?;
        let table = "test_alter_table_add";

        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&mut *tx)
            .await?;
        sqlx::query(&format!(
            "CREATE TABLE {table} (id INTEGER PRIMARY KEY, name TEXT);"
        ))
        .execute(&mut *tx)
        .await?;

        let existing_columns = vec![
            InternalColumnDef::new("id".to_string(), PostgresType::i32, true, true),
            InternalColumnDef::new("name".to_string(), PostgresType::String, false, false),
        ];

        let required_columns = vec![
            ColumnDefinition::new("id", PostgresType::i32, true, true),
            ColumnDefinition::new("name", PostgresType::String, false, false),
            ColumnDefinition::new("email", PostgresType::String, false, false),
        ];

        alter_table(&mut tx, &required_columns, &existing_columns, table).await?;

        let rows = sqlx::query(&format!(
            "SELECT column_name FROM information_schema.columns WHERE table_name = '{table}';"
        ))
        .fetch_all(&mut *tx)
        .await?;

        let column_names: Vec<String> = rows.iter().map(|r| r.get::<String, usize>(0))
        .collect();
        assert!(column_names.contains(&"email".to_string()));

        tx.rollback().await?;

        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&pool)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_alter_table_with_errors() -> Result<(), DataBaseError> {
             let pool = PgPool::connect(
            "postgres://devtest:test@localhost/devtest?sslmode=prefer").await?;
        let mut tx = pool.begin().await?;
        let table = "test_alter_table_error";

        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&mut *tx)
            .await?;
        sqlx::query(&format!(
            "CREATE TABLE {table} (id INTEGER PRIMARY KEY, name TEXT NOT NULL);"
        ))
        .execute(&mut *tx)
        .await?;

        let existing_columns = vec![
            InternalColumnDef::new("id".to_string(), PostgresType::i32, true, true),
            InternalColumnDef::new("name".to_string(), PostgresType::String, true, false),
        ];

        let required_columns = vec![
            ColumnDefinition::new("id", PostgresType::i32, true, true),
            ColumnDefinition::new("name", PostgresType::String, false, false), // NOT NULL mismatch
            ColumnDefinition::new("email", PostgresType::String, false, false),
        ];

        let result = alter_table(&mut tx, &required_columns, &existing_columns, table).await;
        assert!(result.is_err());

        if let Err(DataBaseError::AlterTableError(msg)) = result {
            assert!(msg.contains("NOT NULL mismatch"));
        } else {
            panic!("Expected AlterTableError");
        }

        tx.rollback().await?;
        sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
            .execute(&pool)
            .await?;
        Ok(())
    }
}
