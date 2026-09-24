use crate::query::types::schema::Table;

pub trait Catalog {
    fn create_table(&self, table: Table) {}
}
