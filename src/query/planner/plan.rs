use std::collections::HashMap;

use crate::{
    error::Result,
    query::{
        enigne::engine::Catalog,
        parser::ats::{self, Direction, Expression},
        planner::planner::Planner,
        types::{schema::Table, values::DefaultValues},
    },
};

#[derive(Debug)]
pub enum Plan {
    CreateTable {
        schema: Table,
    },
    DropTable {
        name: String,
    },
    Delete {
        table: String,
        primary_key: usize,
    },
    Insert {
        table: String,
        column: Option<HashMap<usize, usize>>,
    },
    Update {
        table: String,
        primary_key: usize,
        expressions: Vec<(usize, Expression)>,
    },
}

impl Plan {
    pub fn build(stmt: ats::Stmt, catalog: &impl Catalog) -> Result<Self> {
        unimplemented!();
    }
}

pub enum Node {
    Aggregate {
        source: Box<Node>,
        group_by: Vec<Expression>,
        aggregates: Vec<Aggregate>,
    },
    Filter {
        source: Box<Node>,
        predicate: Expression,
    },
    IndexLookup {
        table: Table,
        column: usize,
        values: Vec<DefaultValues>,
        alias: Option<String>,
    },
    HashJoin {
        left: Box<Node>,
        right: Box<Node>,
        left_column: usize,
        right_column: usize,
        outer: bool,
    },
    KeyLookup {
        table: Table,
        values: Vec<DefaultValues>,
        alias: Option<String>,
    },
    Limit {
        sourse: Box<Node>,
        limit: usize,
    },
    Offset {
        sourse: Box<Node>,
        limit: usize,
    },
    NestedLoopJoin {
        left: Box<Node>,
        right: Box<Node>,
        predicate: Option<Expression>,
        outer: bool,
    },
    Order {
        source: Box<Node>,
        key: Vec<(Expression, Direction)>,
    },
    Projection {
        sourse: Box<Node>,
        expressions: Vec<Expression>,
        aliases: Vec<String>,
    },
    Values {
        rows: Vec<Vec<Expression>>,
    },
    Remap {
        source: Box<Node>,
        targets: Vec<Option<usize>>,
    },
    Scan {
        table: Table,
        filter: Option<Expression>,
        alias: Option<String>,
    },
}

pub enum Aggregate {
    Count(Expression),
    Sum(Expression),
    Avg(Expression),
    Max(Expression),
    Min(Expression),
}

impl Aggregate {}
