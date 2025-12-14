///


// schema.rs
#[derive(Debug)]
pub struct ColumnSchema {
    pub name: &'static str,
    pub sql_type: &'static str,
    pub primary: bool,
    pub unique: bool,
    pub not_null: bool,
    pub uuid: bool,
    
}

#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: &'static str,
    pub columns: &'static [ColumnSchema],
}

