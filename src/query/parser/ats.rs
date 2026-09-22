use crate::query::types::values::DataType;

pub enum Stmt {
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
        select: Vec<(Expression, String)>,
        from: Vec<From>,
        t_where: Option<Expression>,
        group_by: Option<Expression>,
        having: Option<Expression>,
        order_by: Option<Expression>,
        offset: Option<Expression>,
        limit: Option<Expression>,
    },
}

pub enum From {
    Table { name: String, alias: Option<String> },
}

#[derive(Debug, Clone)]
pub enum Expression {
    All,
    Column(Option<String>, String),
    Literal(Literal),
    Operator(Operator),
}

#[derive(Clone, Debug)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i32),
    Float(f32),
    String(String),
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

#[derive(Debug, Clone)]
pub enum Operator {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Equal(Box<Expression>, Box<Expression>),
    GreaterThan(Box<Expression>, Box<Expression>),
    GreaterThanOrEqual(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    LessThanOrEqual(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    NotEqual(Box<Expression>),
    Is(Box<Expression>, Literal),
    Like(Box<Expression>, Box<Expression>),
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
                | Equal(lhs, rhs)
                | GreaterThan(lhs, rhs)
                | GreaterThanOrEqual(lhs, rhs)
                | LessThan(lhs, rhs)
                | LessThanOrEqual(lhs, rhs)
                | Like(lhs, rhs) => lhs.walk(visitor) & rhs.walk(visitor),

                Is(ex, _) | NotEqual(ex) | Not(ex) => ex.walk(visitor),
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
                | Equal(lhs, rhs)
                | GreaterThan(lhs, rhs)
                | GreaterThanOrEqual(lhs, rhs)
                | LessThan(lhs, rhs)
                | LessThanOrEqual(lhs, rhs)
                | Like(lhs, rhs) => {
                    lhs.collect(visitor, exprs);
                    rhs.collect(visitor, exprs);
                }

                Is(ex, _) | NotEqual(ex) | Not(ex) => ex.collect(visitor, exprs),
            },

            Self::All | Self::Column(_, _) | Self::Literal(_) => {}
        }
    }
}
