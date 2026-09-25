use crate::{
    error::{Error, Result},
    query::{
        enigne::engine::Catalog,
        parser::ats::{self, Expression, Stmt},
        planner::plan::Plan,
        types::{
            schema::{Column, Table},
            values::DefaultValues,
        },
    },
};

pub struct Planner<'a, C: Catalog> {
    catalog: &'a C,
}

impl<'a, C: Catalog> Planner<'a, C> {
    pub fn new(catalog: &'a C) -> Self {
        Self { catalog: catalog }
    }

    fn build(&mut self, stmt: ats::Stmt) -> Result<Plan> {
        match stmt {
            Stmt::Create { name, columns } => self.build_create_table_plan(name, columns),
            Stmt::Begin | Stmt::Commit | Stmt::Rollback | Stmt::Explain(_) => {
                panic!("unexpected statement {stmt:?}")
            }
            Stmt::DropTable { table } => self.build_drop_table_plan(table),
            Stmt::Delete {
                table,
                where_clause,
            } => self.build_delete_plan(table, where_clause),
            Stmt::Insert {
                table,
                columns,
                values,
            } => unimplemented!(),
            Stmt::Select {
                select,
                from,
                t_where,
                group_by,
                having,
                order_by,
                offset,
                limit,
            } => unimplemented!(),
            Stmt::Update {
                table,
                set,
                r_where,
            } => unimplemented!(),
        }
    }

    fn build_create_table_plan(&mut self, name: String, columns: Vec<ats::Column>) -> Result<Plan> {
        let Some(pm_key) = columns.iter().position(|i| i.primary_key) else {
            return Err(Error::InvalidInput(format!(
                "no primary key for table {name}"
            )));
        };

        if columns.iter().filter(|c| c.primary_key).count() > 1 {
            return Err(Error::InvalidInput(format!(
                "multiple primary keys for table {name}"
            )));
        }

        let columns = columns
            .into_iter()
            .map(|c| {
                let nullable = c.nullable.unwrap_or(!c.primary_key);
                Ok(Column {
                    name: c.name,
                    datatype: c.datatype,
                    nullable,
                    default: match c.default {
                        Some(expr) => Some(Self::build_constant_value(expr)?),
                        None if nullable => Some(DefaultValues::Null),
                        None => None,
                    },
                    unique: c.unique || c.primary_key,
                    index: (c.index || c.unique || c.references.is_some()) && !c.primary_key,
                    references: c.references,
                })
            })
            .collect::<Result<_>>()?;

        Ok(Plan::CreateTable {
            schema: Table {
                name,
                primary_kay: pm_key,
                column: columns,
            },
        })
    }

    fn build_drop_table_plan(&mut self, table: String) -> Result<Plan> {
        Ok(Plan::DropTable { name: table })
    }

    fn build_delete_plan(
        &mut self,
        table: String,
        where_clause: Option<ats::Expression>,
    ) -> Result<Plan> {
        let table = self.catalog.get_table(&table)?;
        unimplemented!();
    }

    fn build_constant_value(expr: ats::Expression) -> Result<DefaultValues> {
        unimplemented!();
    }
}

#[derive(Debug, Default)]
pub struct Scope {}
