use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    error::Result,
    query::{
        enigne::engine::Catalog,
        parser::ats::{self, Direction},
        planner::{context::Context, plan::Plan},
        types::{
            expression::{self, Expression},
            schema::Column,
            values::{DefaultValues, Label},
        },
    },
};

pub struct Planner<'a, C: Catalog> {
    catalog: &'a C,
}

impl<'a, C: Catalog> Planner<'a, C> {
    pub fn new(c: &'a C) -> Self {
        Self { catalog: c }
    }

    pub fn build(&mut self, stmt: ats::Stmt) -> Result<Plan> {
        use ats::Stmt::*;
        match stmt {
            Select {
                select,
                from,
                t_where,
                group_by,
                having,
                order_by,
                offset,
                limit,
            } => self.build_select_table_planner(
                select, from, t_where, group_by, having, offset, limit, order_by,
            ),
            Insert {
                table,
                columns,
                values,
            } => self.build_insert_planner(table, columns, values),
            Delete {
                table,
                where_clause,
            } => self.build_delete_planner(table, where_clause),
            DropTable { table } => self.build_drop_planner(table),
            Create { name, columns } => self.build_create_table_planner(name, columns),
            Update {
                table,
                set,
                r_where,
            } => self.build_update_table_planner(table, set, r_where),

            Begin | Commit | Rollback | Explain(_) => panic!("unexpexted"),
        }
    }

    fn build_select_table_planner(
        &mut self,
        select: Vec<(ats::Expression, Option<String>)>,
        from: Vec<ats::From>,
        t_where: Option<ats::Expression>,
        group_by: Vec<ats::Expression>,
        having: Option<ats::Expression>,
        offset: Option<ats::Expression>,
        limit: Option<ats::Expression>,
        order_by: Vec<(ats::Expression, Direction)>,
    ) -> Result<Plan> {
        unimplemented!()
    }

    fn build_insert_planner(
        &mut self,
        table: String,
        columns: Option<Vec<String>>,
        values: Vec<Vec<ats::Expression>>,
    ) -> Result<Plan> {
        unimplemented!()
    }

    fn build_delete_planner(
        &mut self,
        table: String,
        where_clause: Option<ats::Expression>,
    ) -> Result<Plan> {
        let table = self.catalog.get_table(&table)?;
        let ctx = Context::from_table(&table)?;
        let filter = where_clause.map(|expr|);
        unimplemented!();
    }

    fn build_drop_planner(&mut self, table: String) -> Result<Plan> {
        Ok(Plan::DropTable { name: table })
    }

    fn build_create_table_planner(
        &mut self,
        name: String,
        columns: Vec<ats::Column>,
    ) -> Result<Plan> {
        let Some(primary_key) = columns.iter().position(|i| i.primary_key) else {
            return Err(crate::error::Error::InvalidInput(format!(
                "{name} table have no primary key"
            )));
        };

        if columns.iter().filter(|i| i.primary_key).count() > 1 {
            return Err(crate::error::Error::InvalidInput(format!(
                "{name} table have multiple primary key"
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
                    unique: c.unique || c.primary_key,
                    index: (c.index || c.unique || c.references.is_some()) && !c.primary_key,
                    references: c.references,
                    default: match c.default {
                        Some(expr) => Some(build_constant_value()?),
                        None if nullable => Some(crate::query::types::values::DefaultValues::Null),
                        None => None,
                    },
                })
            })
            .collect::<Result<_>>()?;

        Ok(Plan::CreateTable {
            schema: crate::query::types::schema::Table {
                name,
                primary_kay: primary_key,
                column: columns,
            },
        })
    }

    fn build_update_table_planner(
        &mut self,
        table: String,
        set: BTreeMap<String, Option<ats::Expression>>,
        r_where: Option<ats::Expression>,
    ) -> Result<Plan> {
        unimplemented!()
    }

    fn build_constant_value(expr: ats::Expression) -> Result<DefaultValues> {
        unimplemented!();
    }

    fn build_expression(expr: ats::Expression, ctx: &Context) -> Result<Expression> {
        use expression::Expression::*;

        if let Some(idx) = ctx.lookup_aggregate(&expr) {
            Ok(Columns(idx))
        }

        let build = |expr: Box<ats::Expression>| -> Result<Box<Expression>> {
            Ok(Box::new(Self::build_expression(*expr, ctx)?))
        };

        Ok(match expr {
            ats::Expression::Literal(l) => Constant(match l {
                ats::Literal::Boolean(b) => DefaultValues::Boolean(b),
                ats::Literal::Float(f) => DefaultValues::Float(f),
                ats::Literal::Integer(i) => DefaultValues::Integer(i),
                ats::Literal::Null => DefaultValues::Null,
                ats::Literal::String(s) => DefaultValues::String(s)
            }),
            ats::Expression::Column(table,name) => {
                Columns(ctx.lookup_column(table.as_deref(), &name)?)
            },
            ats::Expression::Operator(op) => match op {
                ats::Operator::Add(lhs,rhs) => Add(build(lhs)?, build(rhs)?),
                ats::Operator::Sub(lhs,rhs) => Subtract(build(lhs)?, build(rhs)?),
                ats::Operator::Multiply(lhs,rhs) => Multiply(build(lhs)?, build(rhs)?),
                ats::Operator::Divide(lhs,rhs) => Divide(build(lhs)?, build(rhs)?),
                ats::Operator::Negate(lhs) => Neglate(build(lhs)?),
                ats::Operator::Eq(lhs,rhs) => Eq(build(lhs)?, build(rhs)?),
                ats::Operator::Gt(lhs,rhs) => Gt(build(lhs)?, build(rhs)?),
                ats::Operator::Lt(lhs,rhs) => Lt(build(lhs)?, build(rhs)?),
                ats::Operator::And(lhs,rhs) => And(build(lhs)?, build(rhs)?),
                ats::Operator::Or(lhs,rhs) => Or(build(lhs)?, build(rhs)?),
                ats::Operator::Gte(lhs, rhs) => Or(Gt(build(lhs)?, build(rhs)?).into(), Eq(build(lhs)?, build(rhs)?).into()),
                ats::Operator::Lte(lhs, rhs) => Or(Lt(build(lhs)?, build(rhs)?).into(), Eq(build(lhs)?, build(rhs)?).into()),
            }

            ats::Expression::All => return Err(crate::error::Error::InvalidData(format!("invalid")))
        })
    }
}
