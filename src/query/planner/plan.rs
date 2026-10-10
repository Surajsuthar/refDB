use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    query::{
        enigne::engine::Catalog,
        parser::ats::{self},
        types::{
            expression::Expression,
            schema::Table,
            values::{DefaultValues, Label},
        },
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
        source: Node,
    },
    Insert {
        table: String,
        column: Option<HashMap<usize, usize>>,
        source: Node,
    },
    Update {
        table: Table,
        primary_key: usize,
        expressions: Vec<(usize, Expression)>,
        source: Node,
    },
    Select(Node),
}

impl Plan {
    pub fn build(stmt: ats::Stmt, catalog: &impl Catalog) -> Result<Self> {
        unimplemented!();
    }
}

#[derive(Debug)]
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
        source: Box<Node>,
        limit: usize,
    },
    Offset {
        source: Box<Node>,
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
    Nothing {
        columns: Vec<Label>,
    },
}

impl Node {
    /// Returns the number of columns emitted by the node.
    pub fn columns(&self) -> usize {
        match self {
            Self::IndexLookup { table, .. }
            | Self::KeyLookup { table, .. }
            | Self::Scan { table, .. } => table.column.len(),

            Self::Filter { source, .. }
            | Self::Limit { source, .. }
            | Self::Offset { source, .. }
            | Self::Order { source, .. } => source.columns(),

            Self::HashJoin { left, right, .. } | Self::NestedLoopJoin { left, right, .. } => {
                left.columns() + right.columns()
            }

            // these nodes modify
            Self::Aggregate {
                aggregates,
                group_by,
                ..
            } => aggregates.len() + group_by.len(),

            Self::Projection { expressions, .. } => expressions.len(),
            Self::Remap { targets, .. } => targets
                .iter()
                .copied()
                .flatten()
                .map(|i| i + 1)
                .max()
                .unwrap_or(0),

            Self::Nothing { columns } => columns.len(),
            Self::Values { rows } => rows.first().map(|row| row.len()).unwrap_or(0),
        }
    }

    pub fn column_label(&self, index: usize) -> Label {
        match self {
            Self::IndexLookup { table, alias, .. }
            | Self::KeyLookup { table, alias, .. }
            | Self::Scan { table, alias, .. } => Label::Qualified(
                alias.as_ref().unwrap_or(&table.name).clone(),
                table.column[index].name.clone(),
            ),

            Self::Aggregate {
                source, group_by, ..
            } => match group_by.get(index) {
                Some(Expression::Columns(index)) => source.column_label(*index),
                Some(_) | None => Label::None,
            },
            Self::Projection {
                sourse,
                expressions,
                aliases,
            } => match aliases.get(index) {
                Some(Label::None) | None => match expressions.get(index) {
                    Some(Expression::Columns(index)) => sourse.column_label(*index),
                    Some(_) | None => Label::None,
                },
                Some(alias) => alias.clone(),
            },
            Self::HashJoin { left, right, .. } | Self::NestedLoopJoin { left, right, .. } => {
                if index < left.columns() {
                    left.column_label(index)
                } else {
                    right.column_label(index - left.columns())
                }
            }

            Self::Filter { source, .. }
            | Self::Limit { source, .. }
            | Self::Offset { source, .. }
            | Self::Order { source, .. } => source.column_label(index),

            Self::Nothing { columns } => columns.get(index).cloned().unwrap_or(Label::None),

            Self::Remap { source, targets } => targets
                .iter()
                .copied()
                .position(|i| i == Some(index))
                .map(|i| source.column_label(i))
                .unwrap_or(Label::None),

            Self::Values { .. } => Label::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Aggregate {
    Count(Expression),
    Sum(Expression),
    Avg(Expression),
    Max(Expression),
    Min(Expression),
}

impl Aggregate {
    // fn format(&self) -> String {
    //     match self {
    //         Self::Avg(expr) => format!("AVG({})", expr.display()),
    //         Self::Sum(expr) => format!("SUM({})", expr.display()),
    //         Self::Count(expr) => format!("COUNT({})", expr.display()),
    //         Self::Max(expr) => format!("MAX({})", expr.display()),
    //         Self::Min(expr) => format!("MIN({})", expr.display()),
    //     }
    // }
    //

    pub fn expr(&self) -> &Expression {
        match self {
            Self::Avg(expr)
            | Self::Sum(expr)
            | Self::Count(expr)
            | Self::Max(expr)
            | Self::Min(expr) => expr,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Ascending => f.write_str("asc"),
            Direction::Descending => f.write_str("dsce"),
        }
    }
}

impl From<ats::Direction> for Direction {
    fn from(value: ats::Direction) -> Self {
        match value {
            ats::Direction::Ascending => Direction::Ascending,
            ats::Direction::Descending => Direction::Descending,
        }
    }
}

impl Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateTable { schema } => write!(f, "CreateTable: {}", schema.name),
            Self::DropTable { name: table, .. } => write!(f, "DropTable: {table}"),
            Self::Delete { table, source, .. } => {
                write!(f, "Delete: {table}")?;
                source.format(f, "", false, true)
            }
            Self::Insert { table, source, .. } => {
                write!(f, "Insert: {}", table)?;
                source.format(f, "", false, true)
            }
            Self::Update {
                table,
                source,
                expressions,
                ..
            } => {
                let expressions = expressions
                    .iter()
                    .map(|(i, expr)| format!("{}={}", table.columns[*i].name, expr.display(source)))
                    .join(", ");
                write!(f, "Update: {} ({expressions})", table.name)?;
                source.format(f, "", false, true)
            }
            Self::Select(root) => root.format(f, "", true, true),
        }
    }
}
