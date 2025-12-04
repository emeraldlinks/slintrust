use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::Column;
use sqlx::Row;
use sqlx::postgres::PgPool;

pub struct QueryBuilder<'a> {
    table: String,
    pool: &'a PgPool,
    selects: Vec<String>,
    wheres: Vec<String>,
    joins: Vec<String>,
    groups: Vec<String>,
    havings: Vec<String>,
    limit_clause: Option<String>,
    offset_clause: Option<String>,
    order_clause: Option<String>,
    params: Vec<String>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new(table: &str, pool: &'a PgPool) -> Self {
        Self {
            table: table.to_string(),
            pool,
            selects: vec!["*".to_string()],
            wheres: vec![],
            joins: vec![],
            groups: vec![],
            havings: vec![],
            limit_clause: None,
            offset_clause: None,
            order_clause: None,
            params: Vec::new(),
        }
    }

    pub fn select(mut self, columns: &[&str]) -> Self {
        self.selects = columns.iter().map(|c| c.to_string()).collect();
        self
    }

    pub fn r#where(mut self, column: &str, op: &str, value: &str) -> Self {
        self.wheres
            .push(format!("{} {} ${}", column, op, self.params.len() + 1));
        self.params.push(value.to_string());
        self
    }

    pub fn like(mut self, column: &str, pattern: &str) -> Self {
    let pattern = format!("%{}%", pattern); // wrap automatically
    self.wheres
        .push(format!("{} LIKE ${}", column, self.params.len() + 1));
    self.params.push(pattern);
    self
}
    pub fn ilike(mut self, column: &str, pattern: &str) -> Self {
    let pattern = format!("%{}%", pattern); // wrap automatically
    self.wheres
        .push(format!("{} ILIKE ${}", column, self.params.len() + 1));
    self.params.push(pattern);
    self
}


    pub fn join(mut self, table: &str, left: &str, right: &str) -> Self {
        self.joins
            .push(format!("JOIN {} ON {} = {}", table, left, right));
        self
    }

    pub fn left_join(mut self, table: &str, left: &str, right: &str) -> Self {
        self.joins
            .push(format!("LEFT JOIN {} ON {} = {}", table, left, right));
        self
    }

    pub fn limit(mut self, n: i64) -> Self {
        self.limit_clause = Some(format!("LIMIT {}", n));
        self
    }

    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_clause = Some(format!("ORDER BY {} {}", column, direction));
        self
    }

    fn build_sql(&self) -> String {
        let mut sql = format!("SELECT {} FROM {}", self.selects.join(","), self.table);
        if !self.joins.is_empty() {
            sql += &format!(" {}", self.joins.join(" "));
        }
        if !self.wheres.is_empty() {
            sql += &format!(" WHERE {}", self.wheres.join(" AND "));
        }
        if !self.groups.is_empty() {
            sql += &format!(" GROUP BY {}", self.groups.join(", "));
        }
        if !self.havings.is_empty() {
            sql += &format!(" HAVING {}", self.havings.join(" AND "));
        }
        if let Some(order) = &self.order_clause {
            sql += &format!(" {}", order);
        }
        if let Some(limit) = &self.limit_clause {
            sql += &format!(" {}", limit);
        }
        if let Some(offset) = &self.offset_clause {
            sql += &format!(" {}", offset);
        }
        sql
    }

   
    pub async fn fetch_all<T>(&self) -> Result<Vec<T>, sqlx::Error>
    where
        T: DeserializeOwned,
    {
        let sql = self.build_sql();
        let mut query = sqlx::query(&sql);
        for param in &self.params {
            query = query.bind(param);
        }

        let rows = query.fetch_all(self.pool).await?;
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

    pub async fn fetch_one<T>(&self) -> Result<T, sqlx::Error>
    where
        T: DeserializeOwned,
    {
        let mut all = self.fetch_all::<T>().await?;
        all.pop().ok_or(sqlx::Error::RowNotFound)
    }
}
