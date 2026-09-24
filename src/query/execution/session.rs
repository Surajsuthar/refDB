use crate::query::types::values::Row;

#[derive(Debug)]
pub enum StmtResult {
    CreateTable { name: String },
    Insert { count: u64 },
    Delete { count: u64 },
    DropTable { name: String },
}
