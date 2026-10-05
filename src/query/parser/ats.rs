use std::{collections::BTreeMap, hash::Hash};

use crate::query::types::values::DataType;

#[derive(Debug)]
pub enum Stmt {
    Begin,
    Commit,
    Rollback,
    Explain(Box<Stmt>),
    Create {
        name: String,
        columns: Vec<Column>,
    },
    Insert {
        table: String,
        columns: Option<Vec<String>>,
        values: Vec<Vec<Expression>>,
    },
    Delete {
        table: String,
        where_clause: Option<Expression>,
    },
    DropTable {
        table: String,
        // if exits for further implementation
    },
    Select {
        select: Vec<(Expression, Option<String>)>,
        from: Vec<From>,
        t_where: Option<Expression>,
        group_by: Vec<Expression>,
        having: Option<Expression>,
        order_by: Vec<(Expression, Direction)>,
        offset: Option<Expression>,
        limit: Option<Expression>,
    },
    Update {
        table: String,
        set: BTreeMap<String, Option<Expression>>,
        r_where: Option<Expression>,
    },
}

#[derive(Debug)]
pub enum From {
    Table {
        name: String,
        alias: Option<String>,
    },
    Join {
        left: Box<From>,
        right: Box<From>,
        j_type: JoinType,
        predicate: Option<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    All,
    Column(Option<String>, String),
    Literal(Literal),
    Operator(Operator),
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Integer(l), Self::Integer(r)) => l == r,
            (Self::Float(l), Self::Float(r)) => l.to_bits() == r.to_bits(),
            (Self::String(l), Self::String(r)) => l == r,
            (_, _) => false,
        }
    }
}

impl Eq for Literal {}

#[derive(Clone, Debug)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i32),
    Float(f32),
    String(String),
}

impl Hash for Literal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Literal::Null => {}
            Literal::Boolean(b) => b.hash(state),
            Literal::Float(f) => f.to_bits().hash(state),
            Literal::Integer(i) => i.hash(state),
            Literal::String(s) => s.hash(state),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum JoinType {
    Left,
    Right,
    Inner,
    Cross,
}

impl JoinType {
    pub fn is_outer(&self) -> bool {
        match self {
            Self::Cross | Self::Inner => false,
            Self::Left | Self::Right => true,
        }
    }
}

#[derive(Debug, Default)]
pub enum Direction {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub datatype: DataType,
    pub primary_key: bool,
    pub nullable: Option<bool>,
    pub default: Option<Expression>,
    pub unique: bool,
    pub index: bool,
    pub references: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operator {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Eq(Box<Expression>, Box<Expression>),
    Gt(Box<Expression>, Box<Expression>),
    Gte(Box<Expression>, Box<Expression>),
    Lt(Box<Expression>, Box<Expression>),
    Lte(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    NotEq(Box<Expression>, Box<Expression>),
    Is(Box<Expression>, Literal),
    Like(Box<Expression>, Box<Expression>),
    Identity(Box<Expression>),
    Negate(Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Remainder(Box<Expression>, Box<Expression>),
    Expo(Box<Expression>, Box<Expression>),
    Add(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Factorial(Box<Expression>),
}

impl Expression {
    pub fn walk(&self, visitor: &mut impl FnMut(&Expression) -> bool) -> bool {
        use Operator::*;

        if !visitor(self) {
            return false;
        }

        match self {
            Self::Operator(op) => match op {
                And(lhs, rhs)
                | Or(lhs, rhs)
                | Eq(lhs, rhs)
                | Gt(lhs, rhs)
                | Gte(lhs, rhs)
                | Lt(lhs, rhs)
                | Lte(lhs, rhs)
                | Like(lhs, rhs) => lhs.walk(visitor) & rhs.walk(visitor),

                Is(ex, _) | NotEq(ex, _) | Not(ex) => ex.walk(visitor),
                Factorial(ex) => ex.walk(visitor),
            },

            Self::All | Self::Column(_, _) | Self::Literal(_) => true,
        }
    }

    pub fn contain(&self, visitor: &mut impl FnMut(&Expression) -> bool) -> bool {
        !self.walk(&mut |expr| !visitor(expr))
    }

    pub fn collect(
        &self,
        visitor: &mut impl FnMut(&Expression) -> bool,
        exprs: &mut Vec<Expression>,
    ) {
        use Operator::*;

        if visitor(self) {
            exprs.push(self.clone());
            return;
        }

        match self {
            Self::Operator(op) => match op {
                And(lhs, rhs)
                | Or(lhs, rhs)
                | Eq(lhs, rhs)
                | Gt(lhs, rhs)
                | Gte(lhs, rhs)
                | Lt(lhs, rhs)
                | Lte(lhs, rhs)
                | Like(lhs, rhs) => {
                    lhs.collect(visitor, exprs);
                    rhs.collect(visitor, exprs);
                }

                Is(ex, _) | NotEq(ex, _) | Not(ex) => ex.collect(visitor, exprs),
            },

            Self::All | Self::Column(_, _) | Self::Literal(_) => {}
        }
    }
}

impl core::convert::From<Literal> for Expression {
    fn from(literal: Literal) -> Self {
        Self::Literal(literal)
    }
}

impl core::convert::From<Operator> for Expression {
    fn from(op: Operator) -> Self {
        Self::Operator(op)
    }
}

impl core::convert::From<Operator> for Box<Expression> {
    fn from(value: Operator) -> Self {
        Box::new(value.into())
    }
}
