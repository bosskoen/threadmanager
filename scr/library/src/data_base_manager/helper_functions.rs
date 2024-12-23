use rusqlite::Transaction;
use super::{ColumnDefinition, DataBaseError};

pub fn generate_placeholder(table_format: &str) -> String {
    table_format
    .split(',')
    .map(|_| "?")
    .collect::<Vec<_>>()
    .join(", ")
}

pub fn create_table(conn: &Transaction, required_columns: &[ColumnDefinition], table_name: &str)-> Result<(), DataBaseError>{
    let mut primary_key = String::new();
    let columns_def = required_columns
    .iter()
    .map(|new_colum| {
    let mut query = format!("{} {}", new_colum.name(), new_colum.col_type());
    if new_colum.not_null(){
        query.push_str(" NOT NULL");
        }
    if new_colum.is_primary_key(){
        if primary_key.len() == 0{
            primary_key.push_str(",\nPRIMARY KEY (");
            primary_key.push_str(new_colum.name());
        }else{
            primary_key.push(',');
            primary_key.push_str(new_colum.name());
        }
        }
    query
    })
    .collect::<Vec<_>>()
    .join(", ");
    if primary_key.len() != 0{
        primary_key.push(')');
    }
    let create_table_query = format!("CREATE TABLE {} ({}{});", table_name, columns_def, primary_key);
    conn.execute(&create_table_query, [])?;
Ok(())
}

pub fn alter_table(conn: &mut Transaction, required_columns: &[ColumnDefinition], existing_columns: &[ColumnDefinition], table_name: &str )-> Result<(), DataBaseError> {
let mut querys: Vec<String> = Vec::new();
let mut errors: Vec<String> = vec!["Errors while updating table fomat:\n".to_string()];
let mut main_error_flag = false;
for required_colum in required_columns {
    match existing_columns.iter().find(|existing_colum| existing_colum.name() == required_colum.name()) {
        Some(existing_colum) => {
            if let Some(error_message) = check_column_mismatch(existing_colum, required_colum, table_name) {
                errors.push(error_message);
                main_error_flag = true;
            }
        },
        None => 
        if required_colum.is_primary_key() {
            main_error_flag = true;
            errors.push(format!("Tryed to add primary key collem: \"{}\" to existing table \"{}\"\n",required_colum.name(), table_name));
        } else {
                
            if required_colum.not_null(){
                querys.push(format!("ALTER TABLE {} ADD COLUMN {} {} NOT NULL;", table_name, required_colum.name(), required_colum.col_type()));
            }else {
                querys.push( format!("ALTER TABLE {} ADD COLUMN {} {};", table_name, required_colum.name(), required_colum.col_type()));
            }
        },               
    }
}
if main_error_flag{
    return Err(DataBaseError::AlterTableError(errors.join("")));
}

for query in querys{
    conn.execute(&query, [])?;
}
Ok(())
}

fn check_column_mismatch(existing_colum: &ColumnDefinition, required_colum: &ColumnDefinition, table_name: &str) -> Option<String>{
    let mut error_flag= false;
    let mut error_mesige = String::new();

    if existing_colum.col_type() != required_colum.col_type(){
        error_flag =true;
        error_mesige.push_str(&format!("- Type mismatch: expected '{}', found '{}'.\n", required_colum.col_type(), existing_colum.col_type()));
    }
    if existing_colum.not_null() != required_colum.not_null(){
        error_flag =true;
        error_mesige.push_str(&format!("- NOT NULL mismatch: expected '{}', found '{}'.\n", required_colum.not_null(), existing_colum.not_null()));
    }
    if existing_colum.is_primary_key() != required_colum.is_primary_key(){
        error_flag =true;
        error_mesige.push_str(&format!("- PRIMARY KEY mismatch: expected '{}', found '{}'.\n", required_colum.is_primary_key(), existing_colum.is_primary_key()));
    }
    if error_flag {
    error_mesige.insert_str(0,&format!("Already existing collum: \"{}\" in table: \"{}\" didn't mach given format:\n", existing_colum.name(), table_name));
    return Some(error_mesige)
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, Result};

    #[test]
    fn test_alter_table() -> Result<()> {
        let mut x = Connection::open_in_memory()?;
        let mut conn = x.transaction()?;

        // Step 1: Create an initial table
        conn.execute(
            "CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT
            );",
            [],
        )?;

        // Step 2: Define required and existing column definitions
        let existing_columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("name", "TEXT", false, false),
        ];

        let required_columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("name", "TEXT", false, false),
            ColumnDefinition::new("email", "TEXT", false, false),
        ];

        // Step 3: Call alter_table
        let result = alter_table(&mut conn, &required_columns, &existing_columns, "test_table");

        // Step 4: Verify results
        assert!(result.is_ok());

        // Check that the "email" column was added
        let mut stmt = conn.prepare("PRAGMA table_info(test_table);")?;
        let mut found_email = false;
        let table_info = stmt.query_map([], |row| {
            let column_name: String = row.get(1)?;
            if column_name == "email" {
                found_email = true;
            }
            Ok(())
        })?;
       for _ in table_info {}

        assert!(found_email);

        Ok(())
    }

    #[test]
    fn test_alter_table_with_errors() -> Result<()> {
        let mut x = Connection::open_in_memory()?;
        let mut conn = x.transaction()?;

        // Step 1: Create an initial table
        conn.execute(
            "CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );",
            [],
        )?;

        // Step 2: Define required and existing column definitions
        let existing_columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("name", "TEXT", true, false),
        ];

        let required_columns = vec![
            ColumnDefinition::new("id", "INTEGER", true, true),
            ColumnDefinition::new("name", "TEXT", false, false), // NOT NULL mismatch
            ColumnDefinition::new("email", "TEXT", false, false),
        ];

        // Step 3: Call alter_table
        let result = alter_table(&mut conn, &required_columns, &existing_columns, "test_table");

        // Step 4: Verify results
        assert!(result.is_err());

        // Check error message
        if let Err(DataBaseError::AlterTableError(msg)) = result {
            assert!(msg.contains("NOT NULL mismatch"));
        } else {
            panic!("Expected AlterTableError");
        }

        Ok(())
    }
}