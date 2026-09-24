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
    fn build(&mut self, stmt: ats::Stmt) -> Result<Plan> {
        match stmt {
            Stmt::Create { name, columns } => self.build_create_table_plan(name, columns),
            Stmt::Begin | Stmt::Commit | Stmt::Rollback | Stmt::Explain(_) => unimplemented!(),
            Stmt::DropTable { table } => unimplemented!(),
            Stmt::Delete {
                table,
                where_clause,
            } => unimplemented!(),
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

    fn build_constant_value(expr: ats::Expression) -> Result<DefaultValues> {
        unimplemented!();
    }
}
