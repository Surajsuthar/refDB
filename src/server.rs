use serde::{Deserialize, Serialize};

use crate::query::{
    execution::session::StmtResult,
    types::{schema::Table, values::Row},
};

/// A SQL client request.
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// Executes a SQL statement.
    Execute(String),
    /// Fetches the given table schema.
    GetTable(String),
    /// Lists all tables.
    ListTables,
    /// Returns server status.
    Status,
}

#[derive(Debug)]
pub enum Response {
    Execute(StmtResult),
    Row(Vec<Row>),
    GetTable(Table),
    ListTable(Vec<String>),
}
