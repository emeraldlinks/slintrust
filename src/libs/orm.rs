use crate::libs::schema::TableSchema;
use crate::query_builder::QueryBuilder;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::Column;
use sqlx::Row;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, query, query_as};
use uuid::Uuid;
use serde_json::Value;

pub struct OrmStruct {
    pub database_url: String,
    pool: Option<PgPool>,
    pub schemas: Vec<TableSchema>,
    limit: u32,
}

impl OrmStruct {
    pub fn new(database_url: String, schemas: Vec<TableSchema>) -> Self {
        println!("Connecting to {}", database_url);
        Self {
            database_url,
            pool: None,
            schemas,
            limit: 0,
        }
    }

   


    pub async fn connect(&mut self) -> sqlx::Result<()> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.database_url)
            .await?;
        self.pool = Some(pool);
        Ok(())
    }

    fn pool(&self) -> &PgPool {
        self.pool.as_ref().expect("Database not connected")
    }

    fn placeholders(count: usize) -> Vec<String> {
        (1..=count).map(|i| format!("${}", i)).collect()
    }

    // -------- Create tables --------
    pub async fn migrate(&self) -> sqlx::Result<()> {
        for schema in &self.schemas {
            let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (", schema.name);
            let cols: Vec<String> = schema
                .columns
                .iter()
                .map(|c| {
                    let mut col_def = format!("{} {}", c.name, c.sql_type);
                    if c.primary {
                        col_def.push_str(" PRIMARY KEY")
                    }
                    if c.unique {
                        col_def.push_str(" UNIQUE")
                    }
                    if c.not_null {
                        col_def.push_str(" NOT NULL")
                    }
                    col_def
                })
                .collect();
            sql.push_str(&cols.join(", "));
            sql.push(')');
            query(&sql).execute(self.pool()).await?;
        }
        Ok(())
    }

    // -------- Insert a record --------
    pub async fn insert<T>(&self, table_name: &str, item: &T) -> sqlx::Result<()>
    where
        T: Serialize,
    {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.name == table_name)
            .expect("Table schema not found");

        let map = serde_json::to_value(item).unwrap();
        let mut cols = Vec::new();
        let mut values: Vec<serde_json::Value> = Vec::new();

        for c in schema.columns.iter() {
            let val = if c.uuid && map.get(&c.name).is_none() {
                serde_json::Value::String(Uuid::new_v4().to_string())
            } else {
                map.get(&c.name).cloned().unwrap_or(serde_json::Value::Null)
            };
            cols.push(c.name);
            values.push(val);
        }

        let placeholders = Self::placeholders(cols.len());
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            schema.name,
            cols.join(","),
            placeholders.join(",")
        );

        let mut query = query(&sql);
        for v in values {
            query = match v {
                serde_json::Value::String(s) => query.bind(s),
                serde_json::Value::Number(n) => query.bind(n.to_string()),
                serde_json::Value::Bool(b) => query.bind(b),
                _ => query.bind(None::<String>),
            };
        }

        query.execute(self.pool()).await?;
        Ok(())
    }

    // -------- Get first record by column --------
    pub async fn first<T>(
    &self,
    table_name: &str,
    column: &str,
    value: &str,
) -> sqlx::Result<Option<T>>
where
    T: DeserializeOwned,
{
    let schema = self
        .schemas
        .iter()
        .find(|s| s.name == table_name)
        .expect("Table schema not found");

    let sql = format!(
        "SELECT * FROM {} WHERE {} = $1 LIMIT 1",
        schema.name, column
    );

    let row = sqlx::query(&sql)
        .bind(value)
        .fetch_optional(self.pool())
        .await?;

    if let Some(r) = row {
        let mut map = serde_json::Map::new();

        for col in r.columns() {
            let col_name = col.name();
            let value = match r.try_get::<Option<i64>, _>(col_name) {
                Ok(Some(v)) => Value::from(v),
                Ok(None) => Value::Null,
                Err(_) => match r.try_get::<Option<f64>, _>(col_name) {
                    Ok(Some(v)) => Value::from(v),
                    Ok(None) => Value::Null,
                    Err(_) => match r.try_get::<Option<bool>, _>(col_name) {
                        Ok(Some(v)) => Value::from(v),
                        Ok(None) => Value::Null,
                        Err(_) => match r.try_get::<Option<String>, _>(col_name) {
                            Ok(Some(v)) => Value::from(v),
                            Ok(None) => Value::Null,
                            Err(_) => Value::Null, // fallback
                        },
                    },
                },
            };
            map.insert(col_name.to_string(), value);
        }

        let obj = serde_json::from_value::<T>(Value::Object(map)).map_err(|e| {
            sqlx::Error::ColumnDecode {
                index: "serde_json".into(),
                source: Box::new(e),
            }
        })?;

        Ok(Some(obj))
    } else {
        Ok(None)
    }
}

     // -------- Fetch multiple records --------
   
pub async fn find<T>(
    &self,
    table_name: &str,
    column: &str,
    filter: &str,
) -> sqlx::Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let schema = self
        .schemas
        .iter()
        .find(|s| s.name == table_name)
        .expect("Table schema not found");

    let sql = format!("SELECT * FROM {} WHERE {} = $1", schema.name, column);
    let rows = sqlx::query(&sql)
        .bind(filter)
        .fetch_all(self.pool())
        .await?;

    let mut result = Vec::with_capacity(rows.len());

    for row in rows {
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let col_name = col.name();
            let value = match row.try_get::<Option<i64>, _>(col_name) {
                Ok(Some(v)) => Value::from(v),
                Ok(None) => Value::Null,
                Err(_) => match row.try_get::<Option<f64>, _>(col_name) {
                    Ok(Some(v)) => Value::from(v),
                    Ok(None) => Value::Null,
                    Err(_) => match row.try_get::<Option<bool>, _>(col_name) {
                        Ok(Some(v)) => Value::from(v),
                        Ok(None) => Value::Null,
                        Err(_) => match row.try_get::<Option<String>, _>(col_name) {
                            Ok(Some(v)) => Value::from(v),
                            Ok(None) => Value::Null,
                            Err(_) => Value::Null,
                        },
                    },
                },
            };
            map.insert(col_name.to_string(), value);
        }

        let obj = serde_json::from_value::<T>(Value::Object(map)).map_err(|e| {
            sqlx::Error::ColumnDecode {
                index: "serde_json".into(),
                source: Box::new(e),
            }
        })?;
        result.push(obj);
    }

    Ok(result)
}



    // -------- Get all records --------
   pub async fn get_all<T>(&self, table_name: &str) -> sqlx::Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let schema = self
        .schemas
        .iter()
        .find(|s| s.name == table_name)
        .expect("Table schema not found");

    let sql = format!("SELECT * FROM {}", schema.name);
    let rows = sqlx::query(&sql).fetch_all(self.pool()).await?;

    let mut results = Vec::with_capacity(rows.len());

    for r in rows {
        let mut map = serde_json::Map::new();
        for col in r.columns() {
            let col_name = col.name();
            let value = match r.try_get::<Option<i64>, _>(col_name) {
                Ok(Some(v)) => Value::from(v),
                Ok(None) => Value::Null,
                Err(_) => match r.try_get::<Option<f64>, _>(col_name) {
                    Ok(Some(v)) => Value::from(v),
                    Ok(None) => Value::Null,
                    Err(_) => match r.try_get::<Option<bool>, _>(col_name) {
                        Ok(Some(v)) => Value::from(v),
                        Ok(None) => Value::Null,
                        Err(_) => match r.try_get::<Option<String>, _>(col_name) {
                            Ok(Some(v)) => Value::from(v),
                            Ok(None) => Value::Null,
                            Err(_) => Value::Null,
                        },
                    },
                },
            };
            map.insert(col_name.to_string(), value);
        }

        let obj = serde_json::from_value::<T>(Value::Object(map)).map_err(|e| {
            sqlx::Error::ColumnDecode {
                index: "serde_json".into(),
                source: Box::new(e),
            }
        })?;
        results.push(obj);
    }

    Ok(results)
}

    // -------- Update record --------
    pub async fn update<T>(
        &self,
        table_name: &str,
        column: &str,
        value: &str,
        item: &T,
    ) -> sqlx::Result<()>
    where
        T: Serialize,
    {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.name == table_name)
            .expect("Table schema not found");

        let map = serde_json::to_value(item).unwrap();
        let mut sets = Vec::new();
        let mut bind_values = Vec::new();

        for c in schema.columns.iter() {
            if let Some(v) = map.get(&c.name) {
                sets.push(format!("{} = ${}", c.name, bind_values.len() + 1));
                bind_values.push(v.clone());
            }
        }

        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ${}",
            schema.name,
            sets.join(", "),
            column,
            bind_values.len() + 1
        );

        let mut query = query(&sql);
        for v in bind_values {
            query = match v {
                serde_json::Value::String(s) => query.bind(s),
                serde_json::Value::Number(n) => query.bind(n.to_string()),
                serde_json::Value::Bool(b) => query.bind(b),
                _ => query.bind(None::<String>),
            };
        }
        query = query.bind(value);

        query.execute(self.pool()).await?;
        Ok(())
    }

    // -------- Delete record --------
    pub async fn delete(&self, table_name: &str, column: &str, value: &str) -> sqlx::Result<()> {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.name == table_name)
            .expect("Table schema not found");

        let sql = format!("DELETE FROM {} WHERE {} = $1", schema.name, column);
        query(&sql).bind(value).execute(self.pool()).await?;
        Ok(())
    }

    // -------- Check if record exists --------
    pub async fn exists(&self, table_name: &str, column: &str, value: &str) -> sqlx::Result<bool> {
        let schema = self
            .schemas
            .iter()
            .find(|s| s.name == table_name)
            .expect("Table schema not found");

        let sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE {} = $1)",
            schema.name, column
        );

        let row: (bool,) = query_as(&sql).bind(value).fetch_one(self.pool()).await?;
        Ok(row.0)
    }

    // -------- Execute raw SQL --------
    pub async fn raw(&self, sql: &str) -> sqlx::Result<sqlx::postgres::PgQueryResult> {
        query(sql).execute(self.pool()).await
    }





      pub fn query<'a>(&'a self, table: &str) -> QueryBuilder<'a> {
    let pool = self.pool.as_ref().expect("DB pool not initialized");
    QueryBuilder::new(table, pool)
}
}






