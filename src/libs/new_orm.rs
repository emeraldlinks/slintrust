use crate::OrmStruct;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{Column, Row};

/// A typed handle to a database table.
pub struct Table<T> {
    orm: OrmStruct,
    name: String,
    key_column: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Table<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    /// Create a new table handle.
    /// ```
    /// let user_table = Table::<User>::new(orm.clone(), "users");
    /// ```
    pub fn new(orm: &OrmStruct, name: &str, key_column: &str) -> Self {
        Self::with_key(orm.to_owned(), name, key_column)
    }

    /// Create a new table handle with a custom key column.
    /// ```
    /// let user_table = Table::<User>::with_key(orm.as_ref(), "users", "user_id");
    /// ```
    pub fn with_key(orm: OrmStruct, name: &str, key_column: &str) -> Self {
        Self {
            orm,
            name: name.to_string(),
            key_column: key_column.to_string(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Insert a new record into the table.
    ///
    /// # Example
    /// ```
    /// let user_table = Table::<User>::new(&orm, "users");
    /// user_table.insert(&new_user).await?;
    /// ```
    pub async fn insert(&self, item: &T) -> sqlx::Result<()> {
        self.orm.insert(&self.name, item).await
    }

    /// Get a single record matching a filter.
    /// Currently supports only single-column equality filters.
    ///
    /// # Example
    /// ```
    /// let user = user_table.get(json!({"id": 1})).await?;
    /// ```
    pub async fn get(&self, filter: Value) -> sqlx::Result<Option<Record<T>>> {
        let map = filter.as_object().expect("Filter must be an object");
        if map.len() != 1 {
            panic!("Only single-column filter supported");
        }
        let (column, value) = map.iter().next().unwrap();
        let obj = self
            .orm
            .first::<T>(&self.name, column, value.as_str().unwrap())
            .await?;
        Ok(obj.map(|o| {
            Record::new(
                self.name.clone(),
                o,
                self.key_column.clone(),
                self.orm.clone(),
            )
        }))
    }

    /// Get all records from the table.
    ///
    /// # Example
    /// ```
    /// let users = user_table.get_all().await?;
    /// ```
    pub async fn get_all(&self) -> sqlx::Result<Vec<Record<T>>> {
        let all = self.orm.get_all::<T>(&self.name).await?;
        Ok(all
            .into_iter()
            .map(|o| {
                Record::new(
                    self.name.clone(),
                    o,
                    self.key_column.clone(),
                    self.orm.clone(),
                )
            })
            .collect())
    }

    /// Create a query builder for advanced queries.
    ///
    /// # Example
    /// ```
    /// let users = user_table
    ///     .query()
    ///     .where_clause("age", ">", "18")
    ///     .order_by("name", "ASC")
    ///     .limit(10)
    ///     .offset(5)
    ///     .distinct()
    ///     .group_by(&["department"])
    ///     .having("count", ">", "1")
    ///     .get()
    ///     .await?;
    /// ```
    pub fn query(&self) -> Query<'_, T> {
        Query::new(self.name.clone(), self.key_column.clone(), &self.orm)
    }
}

/// Represents a single record with instance-level update/delete.
pub struct Record<T> {
    pub table_name: String,
    pub value: T,
    key_column: String,
    orm: OrmStruct,
    id: serde_json::Value,
}

impl<T> Record<T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    pub fn new(table_name: String, value: T, key_column: String, orm: OrmStruct) -> Self {
        let id = serde_json::to_value(&value)
            .unwrap()
            .get(&key_column)
            .unwrap()
            .clone();
        Self {
            table_name,
            value,
            key_column,
            orm,
            id,
        }
    }

    /// Update the current record with changes.
    ///
    /// # Example
    /// ```
    /// record.update(json!({"name": "Joe"})).await?;
    /// ```
    pub async fn update(&self, updates: serde_json::Value) -> sqlx::Result<T> {
        let map = updates.as_object().expect("updates must be an object");
        let mut sets = Vec::new();
        let mut values: Vec<String> = Vec::new();
        for (key, value) in map {
            sets.push(format!("{} = ${}", key, values.len() + 1));
            values.push(value.as_str().expect("value must be string").to_string());
        }
        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ${}",
            self.table_name,
            sets.join(", "),
            self.key_column,
            values.len() + 1
        );
        let mut query = sqlx::query(&sql);
        for value in &values {
            query = query.bind(value);
        }
        query = query.bind(self.id.as_str().unwrap());
        query.execute(self.orm.pool.as_ref().unwrap()).await?;

        // Fetch the updated record
        let updated = self
            .orm
            .query(&self.table_name)
            .r#where(&self.key_column, "=", self.id.as_str().unwrap())
            .fetch_one()
            .await?;
        Ok(updated)
    }

    /// Delete the current record from the table.
    ///
    /// # Example
    /// ```
    /// record.delete().await?;
    /// ```
    pub async fn delete(&self) -> sqlx::Result<()> {
        let id = serde_json::to_value(&self.value)
            .unwrap()
            .get(&self.key_column)
            .expect("key field required")
            .as_str()
            .expect("key must be string")
            .to_string();

        self.orm
            .delete(&self.table_name, &self.key_column, &id)
            .await
    }
}

/// Query builder for advanced queries with WHERE, LIMIT, ORDER BY.
pub struct Query<'a, T> {
    table_name: String,
    key_column: String,
    orm: &'a OrmStruct,
    wheres: Vec<(String, String, String)>,
    limit: Option<u32>,
    offset: Option<u32>,
    order_by: Option<(String, String)>,
    distinct: bool,
    group_by: Vec<String>,
    havings: Vec<(String, String, String)>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T> Query<'a, T>
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    pub fn new(table_name: String, key_column: String, orm: &'a OrmStruct) -> Self {
        Self {
            table_name,
            key_column,
            orm,
            wheres: Vec::new(),
            limit: None,
            offset: None,
            order_by: None,
            distinct: false,
            group_by: Vec::new(),
            havings: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn where_clause(mut self, column: &str, op: &str, value: &str) -> Self {
        self.wheres
            .push((column.to_string(), op.to_string(), value.to_string()));
        self
    }

    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some((column.to_string(), direction.to_string()));
        self
    }

    pub fn offset(mut self, n: u32) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|c| c.to_string()).collect();
        self
    }

    pub fn having(mut self, column: &str, op: &str, value: &str) -> Self {
        self.havings
            .push((column.to_string(), op.to_string(), value.to_string()));
        self
    }

    pub async fn get(self) -> sqlx::Result<Vec<Record<T>>> {
        let select_clause = if self.distinct {
            "SELECT DISTINCT *"
        } else {
            "SELECT *"
        };
        let mut sql = format!("{} FROM {}", select_clause, self.table_name);

        if !self.wheres.is_empty() {
            // Generate numbered placeholders $1, $2, $3...
            let conds: Vec<String> = self
                .wheres
                .iter()
                .enumerate()
                .map(|(i, (c, op, _))| format!("{} {} ${}", c, op, i + 1))
                .collect();
            sql.push_str(&format!(" WHERE {}", conds.join(" AND ")));
        }

        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        if !self.havings.is_empty() {
            let conds: Vec<String> = self
                .havings
                .iter()
                .enumerate()
                .map(|(i, (c, op, _))| format!("{} {} ${}", c, op, self.wheres.len() + i + 1))
                .collect();
            sql.push_str(&format!(" HAVING {}", conds.join(" AND ")));
        }

        if let Some((col, dir)) = &self.order_by {
            sql.push_str(&format!(" ORDER BY {} {}", col, dir));
        }

        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }

        if let Some(off) = self.offset {
            sql.push_str(&format!(" OFFSET {}", off));
        }

        let mut query = sqlx::query(&sql);
        for (_, _, val) in &self.wheres {
            query = query.bind(val);
        }
        for (_, _, val) in &self.havings {
            query = query.bind(val);
        }

        let rows = query.fetch_all(self.orm.pool()).await?;
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
            results.push(Record::new(
                self.table_name.clone(),
                obj,
                self.key_column.clone(),
                self.orm.clone(),
            ));
        }
        Ok(results)
    }

    pub async fn first(self) -> sqlx::Result<Option<Record<T>>> {
        let query = self.limit(1).get().await?;
        Ok(query.into_iter().next())
    }

    pub async fn first_value(self) -> Result<T, sqlx::Error> {
        let query = self.limit(1).get().await?;
        Ok(query
            .into_iter()
            .next()
            .ok_or_else(|| sqlx::Error::RowNotFound)?
            .value)
    }
}
