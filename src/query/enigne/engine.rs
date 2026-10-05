use crate::{
    error::Result,
    query::types::{
        schema::Table,
        values::{DefaultValues, Row},
    },
};

pub trait Transaction: Catalog {
    fn commit(&self) -> Result<()>;
    fn rollback(&self) -> Result<()>;

    fn insert(&self, table: &str, data: Vec<Row>) -> Result<()>;
    fn delete(&self, table: &str, id: u64) -> Result<()>;
    fn update(&self, table: &str, id: u64, data: &[u8]) -> Result<()>;
    fn get(&self, table: &str, id: &[DefaultValues]) -> Result<Vec<u8>>;
}

pub trait Catalog {
    fn create_table(&self, table: Table) -> Result<()>;
    fn delete_table(&self, table: Table) -> Result<()>;
    fn list_tables(&self) -> Result<Vec<Table>>;
    fn get_table(&self, name: &str) -> Result<Table>;
}
