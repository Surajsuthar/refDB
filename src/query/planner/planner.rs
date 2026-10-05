use std::collections::BTreeMap;

use crate::{
    error::Result,
    query::{
        enigne::engine::Catalog,
        parser::ats::{self, Direction},
        planner::plan::Plan,
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
        unimplemented!()
    }

    fn build_drop_planner(&mut self, table: String) -> Result<Plan> {
        unimplemented!()
    }

    fn build_create_table_planner(
        &mut self,
        name: String,
        columns: Vec<ats::Column>,
    ) -> Result<Plan> {
        unimplemented!()
    }

    fn build_update_table_planner(
        &mut self,
        table: String,
        set: BTreeMap<String, Option<ats::Expression>>,
        r_where: Option<ats::Expression>,
    ) -> Result<Plan> {
        unimplemented!()
    }
}
