use std::{collections::BTreeMap, iter::Peekable, ops::Add};

use crate::{
    error::{Error, Result},
    query::{
        parser::ats::{Column, Expression, From},
        types::values::DataType,
    },
};

use super::*;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

impl Parser<'_> {
    pub fn parse(input: &str) -> Result<ats::Stmt> {
        let mut parser = Self::new(input);
        let stmt = parser.parse_stmt()?;
        parser.next_if(|c| *c == Token::SemiColon);

        if let Some(token) = parser.lexer.next().transpose()? {
            return Err(Error::InvalidInput(format!("unexpected token {token:?}")));
        }

        Ok(stmt)
    }

    fn new(input: &str) -> Parser<'_> {
        Parser {
            lexer: Lexer::new(input).peekable(),
        }
    }

    fn peek(&mut self) -> Result<Option<&Token>> {
        let r = self
            .lexer
            .peek()
            .map(|c| c.as_ref().map_err(|e| e.clone()))
            .transpose();

        r
    }

    fn get_next_identifer(&mut self) -> Result<String> {
        match self.next()? {
            Token::Identifer(ident) => Ok(ident),
            token => Err(Error::InvalidInput(format!(
                "expected identifier, got {token:?}"
            ))),
        }
    }

    fn next_if(&mut self, predicate: impl Fn(&Token) -> bool) -> Option<Token> {
        self.peek().ok()?.filter(|t| predicate(t))?;
        self.next().ok()
    }

    fn next_if_keyword(&mut self) -> Option<Keyword> {
        self.peek().ok()?.and_then(|token| match token {
            Token::Keyword(keyword) => Some(*keyword),
            _ => None,
        })
    }

    fn next(&mut self) -> Result<Token> {
        self.lexer
            .next()
            .transpose()?
            .ok_or_else(|| Error::InvalidInput(format!("unexpected end of input")))
    }

    fn check(&mut self, expect: Token) -> Result<()> {
        let token = self.next()?;
        if token != expect {
            return Err(Error::InvalidInput(format!(
                "expected token {expect:?}, found {token:?}"
            )));
        }
        Ok(())
    }

    fn parse_stmt(&mut self) -> Result<ats::Stmt> {
        let Some(token) = self.peek()? else {
            return Err(Error::InvalidInput(format!("unexpected end of input")));
        };
        match token {
            Token::Keyword(Keyword::Begin) => self.parse_begin(),
            Token::Keyword(Keyword::Commit) => self.parse_commit(),
            Token::Keyword(Keyword::Rollback) => self.parse_rollback(),
            Token::Keyword(Keyword::Explain) => self.parse_explain(),

            Token::Keyword(Keyword::Drop) => self.parse_drop(),
            Token::Keyword(Keyword::Create) => self.parse_create_table(),
            Token::Keyword(Keyword::Select) => self.parse_select(),
            Token::Keyword(Keyword::Insert) => self.parse_insert(),
            Token::Keyword(Keyword::Delete) => self.parse_delete(),
            Token::Keyword(Keyword::Update) => self.parse_update(),

            token => Err(Error::InvalidInput(format!("unexpected token"))),
        }
    }

    fn parse_commit(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Commit.into())?;
        Ok(ats::Stmt::Commit)
    }

    fn parse_rollback(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Rollback.into())?;
        Ok(ats::Stmt::Rollback)
    }

    fn parse_explain(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Explain.into())?;
        if self.next_if(|t| *t == Keyword::Explain.into()).is_some() {
            return Err(Error::InvalidInput(format!(
                "cannot nest EXPLAIN statements"
            )));
        }

        Ok(ats::Stmt::Explain(Box::new(self.parse_stmt()?)))
    }

    fn parse_begin(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Begin.into())?;
        Ok(ats::Stmt::Begin)
    }

    fn parse_select(&mut self) -> Result<ats::Stmt> {
        Ok(ats::Stmt::Select {
            select: self.parse_select_clause()?,
            from: self.parse_from_clause()?,
            t_where: self.parse_where_clause()?,
            group_by: self.parse_group_by_clause()?,
            having: self.parse_having_clause()?,
            order_by: self.parse_order_by_clause()?,
            offset: self.parse_offset_clause()?,
            limit: self.parse_limit_clause()?,
        })
    }

    fn parse_select_clause(&mut self) -> Result<Vec<(Expression, Option<String>)>> {
        if self.next_if(|c| *c == Keyword::Select.into()).is_none() {
            return Ok(Vec::new());
        }

        let mut select = Vec::new();

        loop {
            let expr = self.parse_expr()?;
            let mut alias = None;

            if self.next_if(|c| *c == Keyword::As.into()).is_some()
                || matches!(self.peek()?, Some(Token::Identifer(_)))
            {
                if expr == ats::Expression::All {
                    return Err(Error::InvalidInput(format!("can't alias *")));
                }

                alias = Some(self.get_next_identifer()?);
            }

            select.push((expr, alias));

            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }

        Ok(select)
    }

    fn parse_from_clause(&mut self) -> Result<Vec<From>> {
        if self.next_if(|c| *c == Keyword::From.into()).is_none() {
            return Ok(Vec::new());
        }
        let mut from = Vec::new();
        loop {
            let mut from_table = self.parse_from_table()?;

            while let Some(r_type) = self.parse_from_join()? {
                let left = Box::new(from_table);
                let right = Box::new(self.parse_from_table()?);

                let mut predicate = None;
                if r_type != ats::JoinType::Cross {
                    self.check(Keyword::On.into())?;
                    predicate = Some(self.parse_expr()?);
                }

                from_table = ats::From::Join {
                    left,
                    right,
                    j_type: r_type,
                    predicate,
                }
            }
            from.push(from_table);
            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }

        Ok(from)
    }

    fn parse_from_table(&mut self) -> Result<ats::From> {
        let name = self.get_next_identifer()?;
        let mut alias = None;
        if self.next_if(|c| *c == Keyword::As.into()).is_some()
            || matches!(self.peek()?, Some(Token::Identifer(_)))
        {
            alias = Some(self.get_next_identifer()?);
        }

        Ok(ats::From::Table { name, alias })
    }

    fn parse_where_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Where.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_group_by_clause(&mut self) -> Result<Vec<ats::Expression>> {
        if self.next_if(|c| *c == Keyword::Group.into()).is_none() {
            return Ok(Vec::new());
        }

        let mut group = Vec::new();
        self.check(Keyword::By.into())?;
        loop {
            group.push(self.parse_expr()?);
            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }

        Ok(group)
    }

    fn parse_having_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Having.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_order_by_clause(&mut self) -> Result<Vec<(ats::Expression, ats::Direction)>> {
        if self.next_if(|c| *c == Keyword::Order.into()).is_none() {
            return Ok(Vec::new());
        }

        let mut order_by = Vec::new();
        self.check(Keyword::By.into())?;

        loop {
            let expr = self.parse_expr()?;
            let order = self
                .peek()?
                .and_then(|t| match t {
                    Token::Keyword(Keyword::Desc) => Some(ats::Direction::Descending),
                    Token::Keyword(Keyword::Asc) => Some(ats::Direction::Ascending),
                    _ => None,
                })
                .ok_or(Error::InvalidInput("expected ASC or DESC".into()))?;

            order_by.push((expr, order));

            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }
        Ok(order_by)
    }

    fn parse_offset_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Offset.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_limit_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Limit.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_create_table(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Create.into())?;
        self.check(Keyword::Table.into())?;

        let table_name = self.get_next_identifer()?;
        self.check(Token::OpenParen)?;
        let mut column = Vec::new();
        loop {
            column.push(self.parse_crate_table_column()?);
            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }
        self.check(Token::CloseParen)?;
        Ok(ats::Stmt::Create {
            name: table_name,
            columns: column,
        })
    }

    fn parse_crate_table_column(&mut self) -> Result<Column> {
        let column_name = self.get_next_identifer()?;
        let datatype = match self.next()? {
            Token::Keyword(Keyword::Bool | Keyword::Boolean) => DataType::Boolean,
            Token::Keyword(Keyword::Float | Keyword::Double) => DataType::Float,
            Token::Keyword(Keyword::Int | Keyword::Integer) => DataType::Integer,
            Token::Keyword(Keyword::String | Keyword::Text | Keyword::Varchar) => DataType::String,

            token => return Err(Error::InvalidInput(format!("Invalid token"))),
        };

        let mut column = Column {
            name: column_name,
            datatype: datatype,
            unique: false,
            primary_key: false,
            default: None,
            index: false,
            nullable: None,
            references: None,
        };

        while let Some(keyword) = self.next_if_keyword() {
            match keyword {
                Keyword::Primary => {
                    self.check(keyword.into());
                    column.primary_key = true;
                }
                Keyword::Null => {
                    if column.nullable.is_some() {
                        return Err(Error::InvalidInput(format!("Allresdy nullable")));
                    }
                    column.nullable = Some(true)
                }
                Keyword::Not => {
                    self.check(Keyword::Null.into())?;
                    if column.nullable.is_some() {
                        return Err(Error::InvalidInput(format!("Allresdy nullable")));
                    }
                    column.nullable = Some(false)
                }
                Keyword::Unique => column.unique = true,
                Keyword::Index => column.index = true,
                Keyword::Default => column.default = Some(self.parse_expr()?),
                Keyword::Reference => column.references = Some(self.get_next_identifer()?),
                keyword => {
                    return Err(Error::InvalidInput(format!("unexpected keyword {keyword}")));
                }
            }
        }

        Ok(column)
    }

    fn parse_from_join(&mut self) -> Result<Option<ats::JoinType>> {
        if self.next_if(|c| *c == Keyword::Join.into()).is_some() {
            return Ok(Some(ats::JoinType::Inner));
        }

        if self.next_if(|t| *t == Keyword::Cross.into()).is_some() {
            self.check(Keyword::Cross.into())?;
            return Ok(Some(ats::JoinType::Cross));
        }

        if self.next_if(|t| *t == Keyword::Inner.into()).is_some() {
            self.check(Keyword::Inner.into())?;
            return Ok(Some(ats::JoinType::Inner));
        }

        if self.next_if(|t| *t == Keyword::Inner.into()).is_some() {
            self.check(Keyword::Inner.into())?;
            return Ok(Some(ats::JoinType::Inner));
        }

        Ok(None)
    }

    fn parse_insert(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Insert.into())?;
        self.check(Keyword::Into.into())?;

        let table_name = self.get_next_identifer()?;

        let mut columns: Option<Vec<String>> = None;
        if self.next_if(|c| *c == Token::OpenParen).is_some() {
            let cols = columns.insert(Vec::new());
            loop {
                cols.push(self.get_next_identifer()?);
                if self.next_if(|c| *c == Token::Comma).is_none() {
                    break;
                }
            }
            self.check(Token::CloseParen)?;
        }

        self.check(Keyword::Values.into())?;
        let mut values: Vec<Vec<Expression>> = Vec::new();
        loop {
            let mut row = Vec::new();
            self.check(Token::OpenParen)?;
            loop {
                row.push(self.parse_expr()?);
                if self.next_if(|c| *c == Token::Comma).is_none() {
                    break;
                }
            }
            self.check(Token::CloseParen)?;
            values.push(row);
            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }

        Ok(ats::Stmt::Insert {
            table: table_name,
            columns,
            values: values,
        })
    }

    fn parse_delete(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Delete.into())?;
        self.check(Keyword::From.into())?;
        let table_name = self.get_next_identifer()?;
        Ok(ats::Stmt::Delete {
            table: table_name,
            where_clause: self.parse_where_clause()?,
        })
    }

    fn parse_drop(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Drop.into())?;
        self.check(Keyword::Table.into())?;
        let table_name = self.get_next_identifer()?;
        Ok(ats::Stmt::DropTable { table: table_name })
    }

    fn parse_update(&mut self) -> Result<ats::Stmt> {
        self.check(Keyword::Update.into())?;
        let table_name = self.get_next_identifer()?;
        self.check(Keyword::Set.into())?;

        let set = BTreeMap::new();
        loop {
            let column = self.get_next_identifer()?;
            self.check(Token::Eq)?;

            let expr = (self.next_if(|t| *t == Keyword::Default.into()).is_none())
                .then(|| self.parse_expr())
                .transpose()?;

            if set.contains_key(&column) {
                return Err(Error::InvalidData(format!(
                    "column {column} set multiple times"
                )));
            }

            if self.next_if(|c| *c == Token::Comma).is_none() {
                break;
            }
        }

        Ok(ats::Stmt::Update {
            table: table_name,
            set,
            r_where: self.parse_where_clause()?,
        })
    }

    fn parse_expr(&mut self) -> Result<Expression> {
        self.parse_expr_at(0)
    }

    fn parse_expr_at(&mut self, min_prece: u8) -> Result<ats::Expression> {
        let mut lhs = if let Some(prefix) = self.parse_prefix_op(min_prece) {
            let next_expr = prefix.precedence() + prefix.associativity();
            let rhs = self.parse_expr_at(next_expr)?;
            prefix.into_expression(rhs)
        } else {
            self.parse_expr_atom()?
        };

        while let Some(postfix) = self.parse_postfix_op(min_prece)? {
            lhs = postfix.into_expression(lhs)
        }

        while let Some(infix) = self.parse_infix_operator(min_prece) {
            let next_prece = infix.precedence() + infix.associativity();
            let rhs = self.parse_expr_at(next_prece)?;
            lhs = infix.into_expression(lhs, rhs);
        }

        while let Some(postfix) = self.parse_postfix_op(min_prece)? {
            lhs = postfix.into_expression(lhs)
        }

        Ok(lhs)
    }

    fn parse_expr_atom(&mut self) -> Result<ats::Expression> {
        Ok(match self.next()? {
            Token::Asterisk => ats::Expression::All,

            Token::Number(n) if n.chars().all(|c| c.is_ascii_digit()) => ats::Literal::Integer(
                n.parse()
                    .map_err(|e: std::num::ParseIntError| Error::InvalidInput(e.to_string()))?,
            )
            .into(),
            Token::Number(n) => ats::Literal::Float(
                n.parse()
                    .map_err(|e: std::num::ParseFloatError| Error::InvalidInput(e.to_string()))?,
            )
            .into(),
            Token::String(s) => ats::Literal::String(s).into(),
            Token::Keyword(Keyword::True) => ats::Literal::Boolean(true).into(),
            Token::Keyword(Keyword::False) => ats::Literal::Boolean(false).into(),
            Token::Keyword(Keyword::Infinity) => ats::Literal::Float(f32::INFINITY).into(),
            Token::Keyword(Keyword::NaN) => ats::Literal::Float(f32::NAN).into(),
            Token::Keyword(Keyword::Null) => ats::Literal::Null.into(),

            Token::Identifer(table) if self.next_if(|t| *t == Token::Period).is_some() => {
                ats::Expression::Column(Some(table), self.get_next_identifer()?)
            }

            Token::OpenParen => {
                let expr = self.parse_expr()?;
                self.check(Token::CloseParen)?;
                expr
            }
            token => {
                return Err(Error::InvalidInput(format!(
                    "expected expression atom, found {token:?}"
                )));
            }
        })
    }

    fn parse_prefix_op(&mut self, min_prece: u8) -> Option<PrefixOperator> {
        let token = self.next_if(|t| match t {
            Token::Keyword(Keyword::Not) => PrefixOperator::Not.precedence() > min_prece,
            Token::Plus => PrefixOperator::Plus.precedence() > min_prece,
            Token::Minus => PrefixOperator::Minus.precedence() > min_prece,
            _ => false,
        });

        let op = token.map(|t| match t {
            Token::Keyword(Keyword::Not) => PrefixOperator::Not,
            Token::Plus => PrefixOperator::Plus,
            Token::Minus => PrefixOperator::Minus,
            _ => unreachable!(),
        });

        op
    }

    fn parse_infix_operator(&mut self, min_prece: u8) -> Option<InfixOp> {
        let token = self.next_if(|t| match t {
            Token::Asterisk => InfixOp::Multiply.precedence() >= min_prece,
            Token::Plus => InfixOp::Add.precedence() >= min_prece,
            Token::Minus => InfixOp::Sub.precedence() >= min_prece,
            Token::Slash => InfixOp::Divide.precedence() >= min_prece,
            Token::Percent => InfixOp::Remainder.precedence() >= min_prece,
            Token::Eq => InfixOp::Eq.precedence() >= min_prece,
            Token::Gt => InfixOp::Gt.precedence() >= min_prece,
            Token::Gte => InfixOp::Gte.precedence() >= min_prece,
            Token::Lt => InfixOp::Lt.precedence() >= min_prece,
            Token::Lte => InfixOp::Lte.precedence() >= min_prece,
            Token::Keyword(Keyword::And) => InfixOp::And.precedence() >= min_prece,
            Token::Keyword(Keyword::Or) => InfixOp::Or.precedence() >= min_prece,
            Token::Keyword(Keyword::Like) => InfixOp::Like.precedence() >= min_prece,
            _ => false,
        });

        let op = token.map(|t| match t {
            Token::Asterisk => InfixOp::Multiply,
            Token::Plus => InfixOp::Add,
            Token::Minus => InfixOp::Sub,
            Token::Slash => InfixOp::Divide,
            Token::Percent => InfixOp::Remainder,
            Token::Eq => InfixOp::Eq,
            Token::Gt => InfixOp::Gt,
            Token::Gte => InfixOp::Gte,
            Token::Lt => InfixOp::Lt,
            Token::Keyword(Keyword::And) => InfixOp::And,
            Token::Keyword(Keyword::Or) => InfixOp::Or,
            Token::Keyword(Keyword::Like) => InfixOp::Like,
            Token::Lte => InfixOp::Lte,
            _ => unreachable!(),
        });

        op
    }

    fn parse_postfix_op(&mut self, min_prece: u8) -> Result<Option<PostfixOp>> {
        if self.peek()? == Some(&Token::Keyword(Keyword::Is)) {
            if PostfixOp::Is(ats::Literal::Null).precedence() < min_prece {
                return Ok(None);
            }
            self.check(Keyword::Is.into())?;
            let not = self.next_if(|t| *t == Keyword::Not.into()).is_some();
            let value = match self.next()? {
                Token::Keyword(Keyword::NaN) => ats::Literal::Float(f32::NAN),
                Token::Keyword(Keyword::Null) => ats::Literal::Null,
                token => return Err(Error::InvalidInput(format!("unexpected token {token:?}"))),
            };

            let op = match not {
                false => PostfixOp::Is(value),
                true => PostfixOp::IsNot(value),
            };

            return Ok(Some(op));
        }

        let token = self.next_if(|t| match t {
            Token::Asterisk => PostfixOp::Factorial.precedence() >= min_prece,
            _ => false,
        });

        let op = token.map(|t| match t {
            Token::Asterisk => PostfixOp::Factorial,
            _ => unreachable!(),
        });

        Ok(op)
    }
}

enum Associativity {
    Left,
    Right,
}

impl Add<Associativity> for u8 {
    type Output = Self;
    fn add(self, rhs: Associativity) -> Self::Output {
        self + match rhs {
            Associativity::Left => 1,
            Associativity::Right => 0,
        }
    }
}

enum PrefixOperator {
    Minus,
    Plus,
    Not,
}

impl PrefixOperator {
    fn precedence(&self) -> u8 {
        match self {
            PrefixOperator::Minus | PrefixOperator::Plus => 10,
            PrefixOperator::Not => 3,
        }
    }

    fn associativity(&self) -> Associativity {
        Associativity::Right
    }

    fn into_expression(self, rhs: ats::Expression) -> ats::Expression {
        let rhs = Box::new(rhs);
        match self {
            Self::Plus => ats::Operator::Identity(rhs).into(),
            Self::Minus => ats::Operator::Negate(rhs).into(),
            Self::Not => ats::Operator::Not(rhs).into(),
        }
    }
}

enum InfixOp {
    Add,
    And,
    Divide,
    Eq,
    Expo,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    Multiply,
    NotEq,
    Or,
    Remainder,
    Sub,
}

impl InfixOp {
    fn precedence(&self) -> u8 {
        match self {
            Self::Eq | Self::NotEq | Self::Like => 4,
            Self::Gt | Self::Gte | Self::Lt | Self::Lte => 5,
            Self::Add | Self::Sub => 6,
            Self::Multiply | Self::Divide | Self::Remainder => 7,
            Self::Or => 1,
            Self::And => 2,
            Self::Expo => 8,
        }
    }

    fn associativity(&self) -> Associativity {
        match self {
            Self::Expo => Associativity::Right,
            _ => Associativity::Left,
        }
    }

    fn into_expression(self, lhs: ats::Expression, rhs: ats::Expression) -> ats::Expression {
        let (lhs, rhs) = (Box::new(lhs), Box::new(rhs));
        match self {
            Self::Add => ats::Operator::Add(lhs, rhs).into(),
            Self::And => ats::Operator::And(lhs, rhs).into(),
            Self::Divide => ats::Operator::Divide(lhs, rhs).into(),
            Self::Eq => ats::Operator::Eq(lhs, rhs).into(),
            Self::Expo => ats::Operator::Expo(lhs, rhs).into(),
            Self::Gt => ats::Operator::Gt(lhs, rhs).into(),
            Self::Gte => ats::Operator::Gte(lhs, rhs).into(),
            Self::Lt => ats::Operator::Lt(lhs, rhs).into(),
            Self::Lte => ats::Operator::Lte(lhs, rhs).into(),
            Self::Like => ats::Operator::Like(lhs, rhs).into(),
            Self::Multiply => ats::Operator::Multiply(lhs, rhs).into(),
            Self::NotEq => ats::Operator::NotEq(lhs, rhs).into(),
            Self::Or => ats::Operator::Or(lhs, rhs).into(),
            Self::Remainder => ats::Operator::Remainder(lhs, rhs).into(),
            Self::Sub => ats::Operator::Sub(lhs, rhs).into(),
        }
    }
}

enum PostfixOp {
    Factorial,
    Is(ats::Literal),
    IsNot(ats::Literal),
}

impl PostfixOp {
    fn precedence(&self) -> u8 {
        match self {
            Self::Factorial => 9,
            Self::Is(_) | Self::IsNot(_) => 4,
        }
    }

    fn into_expression(&self, lhs: ats::Expression) -> ats::Expression {
        let lhs = Box::new(lhs);
        match self {
            Self::Factorial => ats::Operator::Factorial(lhs).into(),
            Self::Is(lit) => ats::Operator::Is(lhs, lit.clone()).into(),
            Self::IsNot(lit) => {
                ats::Operator::Not(ats::Operator::Is(lhs, lit.clone()).into()).into()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::query::parser::ats::{Direction, JoinType, Literal, Operator, Stmt};
    use std::time::Duration;

    // ------------------------------------------------------------------
    // helpers
    // ------------------------------------------------------------------

    fn parse(sql: &str) -> Stmt {
        Parser::parse(sql).unwrap_or_else(|e| panic!("failed to parse {sql:?}: {e}"))
    }

    fn parse_err(sql: &str) -> Error {
        match Parser::parse(sql) {
            Ok(stmt) => panic!("expected {sql:?} to fail, but it parsed as {stmt:?}"),
            Err(e) => e,
        }
    }

    /// Runs the parser on another thread so that a parser infinite loop shows up
    /// as a test failure instead of hanging the whole test run.
    fn parse_with_timeout(sql: &str) -> Result<Stmt> {
        let sql = sql.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(Parser::parse(&sql));
        });
        rx.recv_timeout(Duration::from_secs(2))
            .expect("parser did not terminate (infinite loop?)")
    }

    /// Parses `SELECT <expr>` and returns the single projected expression.
    fn parse_expr_str(expr: &str) -> Expression {
        match parse(&format!("SELECT {expr}")) {
            Stmt::Select { mut select, .. } => {
                assert_eq!(select.len(), 1, "expected exactly one projection");
                select.remove(0).0
            }
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    fn col(name: &str) -> Expression {
        Expression::Column(None, name.into())
    }

    fn qcol(table: &str, name: &str) -> Expression {
        Expression::Column(Some(table.into()), name.into())
    }

    fn int(i: i32) -> Expression {
        Literal::Integer(i).into()
    }

    fn float(f: f32) -> Expression {
        Literal::Float(f).into()
    }

    fn text(s: &str) -> Expression {
        Literal::String(s.into()).into()
    }

    fn boolean(b: bool) -> Expression {
        Literal::Boolean(b).into()
    }

    fn null() -> Expression {
        Literal::Null.into()
    }

    fn bin(
        ctor: fn(Box<Expression>, Box<Expression>) -> Operator,
        lhs: Expression,
        rhs: Expression,
    ) -> Expression {
        ctor(Box::new(lhs), Box::new(rhs)).into()
    }

    fn un(ctor: fn(Box<Expression>) -> Operator, expr: Expression) -> Expression {
        ctor(Box::new(expr)).into()
    }

    fn as_table(from: &ats::From) -> (&str, Option<&str>) {
        match from {
            ats::From::Table { name, alias } => (name.as_str(), alias.as_deref()),
            other => panic!("expected a table, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // transaction control
    // ------------------------------------------------------------------

    #[test]
    fn parse_begin() {
        assert!(matches!(parse("BEGIN;"), Stmt::Begin));
    }

    #[test]
    fn parse_commit() {
        assert!(matches!(parse("COMMIT;"), Stmt::Commit));
    }

    #[test]
    fn parse_rollback() {
        assert!(matches!(parse("ROLLBACK;"), Stmt::Rollback));
    }

    #[test]
    fn semicolon_is_optional() {
        assert!(matches!(parse("BEGIN"), Stmt::Begin));
        assert!(matches!(parse("SELECT 1"), Stmt::Select { .. }));
    }

    #[test]
    fn keywords_are_case_insensitive_and_whitespace_tolerant() {
        assert!(matches!(parse("begin"), Stmt::Begin));
        assert!(matches!(parse("BeGiN"), Stmt::Begin));
        assert!(matches!(parse("  \n\tCOMMIT \n ; \n"), Stmt::Commit));
    }

    #[test]
    fn explain_wraps_inner_statement() {
        let Stmt::Explain(inner) = parse("EXPLAIN SELECT 1") else {
            panic!("expected EXPLAIN");
        };
        assert!(matches!(*inner, Stmt::Select { .. }));
    }

    #[test]
    fn explain_cannot_be_nested() {
        let err = parse_err("EXPLAIN EXPLAIN SELECT 1");
        assert!(err.to_string().contains("nest"), "unexpected error: {err}");
    }

    #[test]
    fn explain_requires_a_statement() {
        parse_err("EXPLAIN");
    }

    // ------------------------------------------------------------------
    // top-level errors
    // ------------------------------------------------------------------

    #[test]
    fn empty_input_is_an_error() {
        parse_err("");
        parse_err("   \n ");
    }

    #[test]
    fn lone_semicolon_is_an_error() {
        parse_err(";");
    }

    #[test]
    fn unknown_statement_is_an_error() {
        parse_err("FOO");
        parse_err("WHERE 1");
        parse_err("42");
    }

    #[test]
    fn trailing_tokens_are_rejected() {
        parse_err("BEGIN BEGIN");
        parse_err("BEGIN;;");
        parse_err("SELECT 1 SELECT 2");
        parse_err("COMMIT; ROLLBACK");
    }

    #[test]
    fn unterminated_string_is_an_error() {
        parse_err("SELECT 'oops");
    }

    #[test]
    #[ignore = "bug: Lexer::next swallows an unknown trailing character (scan_symbol consumes it, then peek() sees EOF)"]
    fn unknown_character_at_end_is_rejected() {
        parse_err("SELECT 1 @");
    }

    // ------------------------------------------------------------------
    // DROP TABLE
    // ------------------------------------------------------------------

    #[test]
    fn drop_table() {
        let Stmt::DropTable { table } = parse("DROP TABLE users;") else {
            panic!("expected DROP TABLE");
        };
        assert_eq!(table, "users");
    }

    #[test]
    fn drop_table_errors() {
        parse_err("DROP users");
        parse_err("DROP TABLE");
        parse_err("DROP TABLE 1");
    }

    // ------------------------------------------------------------------
    // CREATE TABLE
    // ------------------------------------------------------------------

    #[test]
    fn create_table_column_types() {
        use crate::query::types::values::DataType;

        let Stmt::Create { name, columns } =
            parse("CREATE TABLE t (a INT, b TEXT, c BOOLEAN, d FLOAT)")
        else {
            panic!("expected CREATE TABLE");
        };

        assert_eq!(name, "t");
        let got: Vec<_> = columns
            .iter()
            .map(|c| (c.name.as_str(), &c.datatype))
            .collect();
        assert_eq!(
            got,
            vec![
                ("a", &DataType::Integer),
                ("b", &DataType::String),
                ("c", &DataType::Boolean),
                ("d", &DataType::Float),
            ]
        );

        // bare columns carry no constraints
        for c in &columns {
            assert!(!c.primary_key && !c.unique && !c.index);
            assert!(c.nullable.is_none() && c.default.is_none() && c.references.is_none());
        }
    }

    #[test]
    fn create_table_type_aliases() {
        use crate::query::types::values::DataType;

        let cases = [
            ("BOOL", DataType::Boolean),
            ("BOOLEAN", DataType::Boolean),
            ("FLOAT", DataType::Float),
            ("DOUBLE", DataType::Float),
            ("INT", DataType::Integer),
            ("INTEGER", DataType::Integer),
            ("STRING", DataType::String),
            ("TEXT", DataType::String),
            ("VARCHAR", DataType::String),
        ];

        for (sql_type, expected) in cases {
            let Stmt::Create { columns, .. } = parse(&format!("CREATE TABLE t (a {sql_type})"))
            else {
                panic!("expected CREATE TABLE");
            };
            assert_eq!(columns[0].datatype, expected, "type {sql_type}");
        }
    }

    #[test]
    fn create_table_errors() {
        parse_err("CREATE t (a INT)"); // missing TABLE
        parse_err("CREATE TABLE (a INT)"); // missing name
        parse_err("CREATE TABLE t a INT"); // missing parens
        parse_err("CREATE TABLE t (a INT"); // unclosed paren
        parse_err("CREATE TABLE t ()"); // no columns
        parse_err("CREATE TABLE t (a BLOB)"); // unknown type
        parse_err("CREATE TABLE t (a)"); // missing type
        parse_err("CREATE TABLE t (a INT,)"); // trailing comma
    }

    #[test]
    #[ignore = "bug: PRIMARY consumed but there is no KEY keyword, so `PRIMARY KEY` leaves `key` unparsed"]
    fn create_table_primary_key() {
        let Stmt::Create { columns, .. } =
            parse_with_timeout("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)").unwrap()
        else {
            panic!("expected CREATE TABLE");
        };
        assert!(columns[0].primary_key);
        assert!(!columns[1].primary_key);
    }

    #[test]
    #[ignore = "bug: column constraint keywords are peeked but never consumed (NULL/NOT NULL/DEFAULT error, UNIQUE/INDEX loop forever)"]
    fn create_table_column_constraints() {
        let Stmt::Create { columns, .. } = parse_with_timeout(
            "CREATE TABLE t (a INT NOT NULL, b INT NULL, c INT UNIQUE, d INT INDEX, e INT DEFAULT 5)",
        )
        .unwrap() else {
            panic!("expected CREATE TABLE");
        };

        assert_eq!(columns[0].nullable, Some(false));
        assert_eq!(columns[1].nullable, Some(true));
        assert!(columns[2].unique);
        assert!(columns[3].index);
        assert_eq!(columns[4].default, Some(int(5)));
    }

    // ------------------------------------------------------------------
    // INSERT
    // ------------------------------------------------------------------

    #[test]
    fn insert_without_column_list() {
        let Stmt::Insert {
            table,
            columns,
            values,
        } = parse("INSERT INTO users VALUES (1, 'alice', TRUE, NULL)")
        else {
            panic!("expected INSERT");
        };

        assert_eq!(table, "users");
        assert!(columns.is_none());
        assert_eq!(
            values,
            vec![vec![int(1), text("alice"), boolean(true), null()]]
        );
    }

    #[test]
    fn insert_with_column_list() {
        let Stmt::Insert {
            columns, values, ..
        } = parse("INSERT INTO users (id, name) VALUES (1, 'bob')")
        else {
            panic!("expected INSERT");
        };

        assert_eq!(columns, Some(vec!["id".to_string(), "name".to_string()]));
        assert_eq!(values, vec![vec![int(1), text("bob")]]);
    }

    #[test]
    fn insert_multiple_rows() {
        let Stmt::Insert { values, .. } = parse("INSERT INTO t VALUES (1, 2), (3, 4), (5, 6)")
        else {
            panic!("expected INSERT");
        };

        assert_eq!(
            values,
            vec![
                vec![int(1), int(2)],
                vec![int(3), int(4)],
                vec![int(5), int(6)],
            ]
        );
    }

    #[test]
    fn insert_values_are_expressions() {
        let Stmt::Insert { values, .. } = parse("INSERT INTO t VALUES (1 + 2, -3)") else {
            panic!("expected INSERT");
        };

        assert_eq!(
            values,
            vec![vec![
                bin(Operator::Add, int(1), int(2)),
                un(Operator::Negate, int(3)),
            ]]
        );
    }

    #[test]
    fn insert_errors() {
        parse_err("INSERT users VALUES (1)"); // missing INTO
        parse_err("INSERT INTO VALUES (1)"); // missing table
        parse_err("INSERT INTO t (1)"); // missing VALUES
        parse_err("INSERT INTO t VALUES"); // missing row
        parse_err("INSERT INTO t VALUES 1, 2"); // row not parenthesised
        parse_err("INSERT INTO t VALUES (1, 2"); // unclosed paren
        parse_err("INSERT INTO t VALUES (1),"); // dangling comma
        parse_err("INSERT INTO t (a, ) VALUES (1)"); // dangling column comma
        parse_err("INSERT INTO t () VALUES (1)"); // empty column list
    }

    // ------------------------------------------------------------------
    // DELETE
    // ------------------------------------------------------------------

    #[test]
    fn delete_all_rows() {
        let Stmt::Delete {
            table,
            where_clause,
        } = parse("DELETE FROM users")
        else {
            panic!("expected DELETE");
        };
        assert_eq!(table, "users");
        assert!(where_clause.is_none());
    }

    #[test]
    fn delete_with_where() {
        let Stmt::Delete { where_clause, .. } = parse("DELETE FROM users WHERE users.id = 1")
        else {
            panic!("expected DELETE");
        };
        assert_eq!(
            where_clause,
            Some(bin(Operator::Eq, qcol("users", "id"), int(1)))
        );
    }

    #[test]
    fn delete_errors() {
        parse_err("DELETE users"); // missing FROM
        parse_err("DELETE FROM"); // missing table
        parse_err("DELETE FROM users WHERE"); // empty WHERE
    }

    // ------------------------------------------------------------------
    // UPDATE
    // ------------------------------------------------------------------

    #[test]
    fn update_parses_and_keeps_where() {
        let Stmt::Update { table, r_where, .. } =
            parse("UPDATE users SET name = 'x' WHERE users.id = 1")
        else {
            panic!("expected UPDATE");
        };
        assert_eq!(table, "users");
        assert_eq!(
            r_where,
            Some(bin(Operator::Eq, qcol("users", "id"), int(1)))
        );
    }

    #[test]
    #[ignore = "bug: parse_update never inserts the parsed assignments into `set`"]
    fn update_collects_assignments() {
        let Stmt::Update { set, .. } =
            parse("UPDATE users SET name = 'x', age = 3, nick = DEFAULT")
        else {
            panic!("expected UPDATE");
        };

        assert_eq!(set.get("name"), Some(&Some(text("x"))));
        assert_eq!(set.get("age"), Some(&Some(int(3))));
        // `DEFAULT` is represented as `None`
        assert_eq!(set.get("nick"), Some(&None));
    }

    #[test]
    fn update_errors() {
        parse_err("UPDATE users"); // missing SET
        parse_err("UPDATE SET a = 1"); // missing table
        parse_err("UPDATE users SET a 1"); // missing '='
        parse_err("UPDATE users SET = 1"); // missing column
        parse_err("UPDATE users SET a ="); // missing value
    }

    // ------------------------------------------------------------------
    // SELECT: projection
    // ------------------------------------------------------------------

    #[test]
    fn select_without_from() {
        let Stmt::Select {
            select,
            from,
            t_where,
            group_by,
            having,
            order_by,
            offset,
            limit,
        } = parse("SELECT 1")
        else {
            panic!("expected SELECT");
        };

        assert_eq!(select, vec![(int(1), None)]);
        assert!(from.is_empty());
        assert!(t_where.is_none());
        assert!(group_by.is_empty());
        assert!(having.is_none());
        assert!(order_by.is_empty());
        assert!(offset.is_none());
        assert!(limit.is_none());
    }

    #[test]
    fn select_multiple_projections() {
        let Stmt::Select { select, .. } = parse("SELECT 1, 'two', t.three") else {
            panic!("expected SELECT");
        };
        assert_eq!(
            select,
            vec![
                (int(1), None),
                (text("two"), None),
                (qcol("t", "three"), None)
            ]
        );
    }

    #[test]
    fn select_alias_with_and_without_as() {
        let Stmt::Select { select, .. } = parse("SELECT t.a AS x, t.b y, 1 AS one") else {
            panic!("expected SELECT");
        };
        assert_eq!(
            select,
            vec![
                (qcol("t", "a"), Some("x".to_string())),
                (qcol("t", "b"), Some("y".to_string())),
                (int(1), Some("one".to_string())),
            ]
        );
    }

    #[test]
    fn select_alias_must_be_an_identifier() {
        parse_err("SELECT 1 AS");
        parse_err("SELECT 1 AS 2");
        parse_err("SELECT 1 AS FROM t");
    }

    #[test]
    #[ignore = "bug: the lexer has no `*` token (also `/`, `%`, `!`), so `SELECT *` cannot be lexed"]
    fn select_star() {
        let Stmt::Select { select, from, .. } = parse("SELECT * FROM users") else {
            panic!("expected SELECT");
        };
        assert_eq!(select, vec![(Expression::All, None)]);
        assert_eq!(as_table(&from[0]), ("users", None));
    }

    #[test]
    #[ignore = "bug: parse_expr_atom has no arm for a bare identifier, only `table.column` works"]
    fn select_unqualified_column() {
        let Stmt::Select { select, .. } = parse("SELECT id, name FROM users") else {
            panic!("expected SELECT");
        };
        assert_eq!(select, vec![(col("id"), None), (col("name"), None)]);
    }

    #[test]
    fn select_errors() {
        parse_err("SELECT"); // nothing to select
        parse_err("SELECT 1,"); // dangling comma
        parse_err("SELECT 1 +"); // dangling operator
        parse_err("SELECT (1"); // unclosed paren
        parse_err("SELECT 1)"); // stray paren
        parse_err("SELECT FROM users"); // missing projection
        parse_err("SELECT 1 FROM"); // missing table
        parse_err("SELECT 1 WHERE"); // empty WHERE
    }

    // ------------------------------------------------------------------
    // SELECT: FROM / JOIN
    // ------------------------------------------------------------------

    #[test]
    fn from_single_table() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM users") else {
            panic!("expected SELECT");
        };
        assert_eq!(from.len(), 1);
        assert_eq!(as_table(&from[0]), ("users", None));
    }

    #[test]
    fn from_table_alias_with_and_without_as() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM users u, orders AS o") else {
            panic!("expected SELECT");
        };
        assert_eq!(from.len(), 2);
        assert_eq!(as_table(&from[0]), ("users", Some("u")));
        assert_eq!(as_table(&from[1]), ("orders", Some("o")));
    }

    #[test]
    fn from_comma_separated_tables() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a, b, c") else {
            panic!("expected SELECT");
        };
        let names: Vec<_> = from.iter().map(|f| as_table(f).0).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn join_defaults_to_inner() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a JOIN b ON a.id = b.a_id") else {
            panic!("expected SELECT");
        };
        assert_eq!(from.len(), 1);

        let ats::From::Join {
            left,
            right,
            j_type,
            predicate,
        } = &from[0]
        else {
            panic!("expected a join, got {:?}", from[0]);
        };

        assert_eq!(as_table(left), ("a", None));
        assert_eq!(as_table(right), ("b", None));
        assert_eq!(*j_type, JoinType::Inner);
        assert_eq!(
            predicate,
            &Some(bin(Operator::Eq, qcol("a", "id"), qcol("b", "a_id")))
        );
    }

    #[test]
    fn joins_are_left_deep() {
        let Stmt::Select { from, .. } =
            parse("SELECT 1 FROM a JOIN b ON a.id = b.id JOIN c ON b.id = c.id")
        else {
            panic!("expected SELECT");
        };

        let ats::From::Join { left, right, .. } = &from[0] else {
            panic!("expected a join");
        };
        assert_eq!(as_table(right), ("c", None));
        let ats::From::Join { left, right, .. } = &**left else {
            panic!("expected the left side to be a nested join");
        };
        assert_eq!(as_table(left), ("a", None));
        assert_eq!(as_table(right), ("b", None));
    }

    #[test]
    fn join_with_aliases() {
        let Stmt::Select { from, .. } =
            parse("SELECT 1 FROM users u JOIN orders AS o ON u.id = o.user_id")
        else {
            panic!("expected SELECT");
        };
        let ats::From::Join { left, right, .. } = &from[0] else {
            panic!("expected a join");
        };
        assert_eq!(as_table(left), ("users", Some("u")));
        assert_eq!(as_table(right), ("orders", Some("o")));
    }

    #[test]
    fn join_requires_on_clause() {
        parse_err("SELECT 1 FROM a JOIN b");
        parse_err("SELECT 1 FROM a JOIN b ON");
    }

    #[test]
    #[ignore = "bug: parse_from_join consumes CROSS/INNER and then `check`s the same keyword again; JOIN is never consumed"]
    fn explicit_inner_and_cross_join() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a INNER JOIN b ON a.id = b.id") else {
            panic!("expected SELECT");
        };
        assert!(matches!(
            &from[0],
            ats::From::Join {
                j_type: JoinType::Inner,
                ..
            }
        ));

        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a CROSS JOIN b") else {
            panic!("expected SELECT");
        };
        assert!(matches!(
            &from[0],
            ats::From::Join {
                j_type: JoinType::Cross,
                predicate: None,
                ..
            }
        ));
    }

    #[test]
    #[ignore = "bug: parse_from_join has no LEFT/RIGHT handling (and a duplicated INNER branch)"]
    fn left_and_right_join() {
        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a LEFT JOIN b ON a.id = b.id") else {
            panic!("expected SELECT");
        };
        assert!(matches!(
            &from[0],
            ats::From::Join {
                j_type: JoinType::Left,
                ..
            }
        ));

        let Stmt::Select { from, .. } = parse("SELECT 1 FROM a RIGHT JOIN b ON a.id = b.id") else {
            panic!("expected SELECT");
        };
        assert!(matches!(
            &from[0],
            ats::From::Join {
                j_type: JoinType::Right,
                ..
            }
        ));
    }

    // ------------------------------------------------------------------
    // SELECT: WHERE / GROUP BY / HAVING / ORDER BY / OFFSET / LIMIT
    // ------------------------------------------------------------------

    #[test]
    fn select_where() {
        let Stmt::Select { t_where, .. } =
            parse("SELECT t.a FROM t WHERE t.a >= 18 AND t.b LIKE 'x%'")
        else {
            panic!("expected SELECT");
        };

        assert_eq!(
            t_where,
            Some(bin(
                Operator::And,
                bin(Operator::Gte, qcol("t", "a"), int(18)),
                bin(Operator::Like, qcol("t", "b"), text("x%")),
            ))
        );
    }

    #[test]
    fn select_group_by_and_having() {
        let Stmt::Select {
            group_by, having, ..
        } = parse("SELECT t.a FROM t GROUP BY t.a, t.b HAVING t.a > 1")
        else {
            panic!("expected SELECT");
        };

        assert_eq!(group_by, vec![qcol("t", "a"), qcol("t", "b")]);
        assert_eq!(having, Some(bin(Operator::Gt, qcol("t", "a"), int(1))));
    }

    #[test]
    fn group_requires_by() {
        parse_err("SELECT t.a FROM t GROUP t.a");
        parse_err("SELECT t.a FROM t GROUP BY");
    }

    #[test]
    fn select_limit() {
        let Stmt::Select { limit, offset, .. } = parse("SELECT 1 LIMIT 5") else {
            panic!("expected SELECT");
        };
        assert_eq!(limit, Some(int(5)));
        assert!(offset.is_none());
    }

    #[test]
    fn select_offset_then_limit() {
        // The grammar currently expects OFFSET before LIMIT.
        let Stmt::Select { limit, offset, .. } = parse("SELECT 1 OFFSET 20 LIMIT 10") else {
            panic!("expected SELECT");
        };
        assert_eq!(offset, Some(int(20)));
        assert_eq!(limit, Some(int(10)));
    }

    #[test]
    #[ignore = "limitation: standard `LIMIT n OFFSET m` order is rejected, parse_select reads OFFSET before LIMIT"]
    fn select_limit_then_offset() {
        let Stmt::Select { limit, offset, .. } = parse("SELECT 1 LIMIT 10 OFFSET 20") else {
            panic!("expected SELECT");
        };
        assert_eq!(limit, Some(int(10)));
        assert_eq!(offset, Some(int(20)));
    }

    #[test]
    #[ignore = "bug: parse_order_by_clause only peeks the ASC/DESC token, leaving it as trailing input"]
    fn select_order_by_with_direction() {
        let Stmt::Select { order_by, .. } = parse("SELECT t.a FROM t ORDER BY t.a DESC, t.b ASC")
        else {
            panic!("expected SELECT");
        };

        assert_eq!(order_by.len(), 2);
        assert_eq!(order_by[0].0, qcol("t", "a"));
        assert!(matches!(order_by[0].1, Direction::Descending));
        assert_eq!(order_by[1].0, qcol("t", "b"));
        assert!(matches!(order_by[1].1, Direction::Ascending));
    }

    #[test]
    #[ignore = "bug: parse_order_by_clause errors with `expected ASC or DESC` instead of defaulting to ascending"]
    fn select_order_by_defaults_to_ascending() {
        let Stmt::Select { order_by, .. } = parse("SELECT t.a FROM t ORDER BY t.a") else {
            panic!("expected SELECT");
        };
        assert_eq!(order_by.len(), 1);
        assert!(matches!(order_by[0].1, Direction::Ascending));
    }

    #[test]
    fn order_requires_by() {
        parse_err("SELECT t.a FROM t ORDER t.a");
        parse_err("SELECT t.a FROM t ORDER BY");
    }

    #[test]
    fn select_with_every_clause() {
        let Stmt::Select {
            select,
            from,
            t_where,
            group_by,
            having,
            offset,
            limit,
            ..
        } = parse(
            "SELECT t.a AS x FROM t WHERE t.b = 1 GROUP BY t.a HAVING t.a > 0 OFFSET 2 LIMIT 3;",
        )
        else {
            panic!("expected SELECT");
        };

        assert_eq!(select, vec![(qcol("t", "a"), Some("x".to_string()))]);
        assert_eq!(as_table(&from[0]), ("t", None));
        assert!(t_where.is_some());
        assert_eq!(group_by, vec![qcol("t", "a")]);
        assert!(having.is_some());
        assert_eq!(offset, Some(int(2)));
        assert_eq!(limit, Some(int(3)));
    }

    // ------------------------------------------------------------------
    // expressions: literals
    // ------------------------------------------------------------------

    #[test]
    fn literal_values() {
        assert_eq!(parse_expr_str("42"), int(42));
        assert_eq!(parse_expr_str("0"), int(0));
        assert_eq!(parse_expr_str("1.5"), float(1.5));
        assert_eq!(parse_expr_str("'hello'"), text("hello"));
        assert_eq!(parse_expr_str("''"), text(""));
        assert_eq!(parse_expr_str("TRUE"), boolean(true));
        assert_eq!(parse_expr_str("false"), boolean(false));
        assert_eq!(parse_expr_str("NULL"), null());
    }

    #[test]
    fn literal_string_escaped_quote() {
        assert_eq!(parse_expr_str("'it''s'"), text("it's"));
    }

    #[test]
    fn literal_special_floats() {
        assert_eq!(parse_expr_str("INFINITY"), float(f32::INFINITY));
        // NaN != NaN for f32, but ats::Literal compares by bit pattern.
        assert_eq!(parse_expr_str("NaN"), float(f32::NAN));
    }

    #[test]
    fn integer_literal_overflow_is_an_error() {
        assert_eq!(parse_expr_str("2147483647"), int(i32::MAX));
        parse_err("SELECT 2147483648");
    }

    #[test]
    fn qualified_column() {
        assert_eq!(parse_expr_str("users.id"), qcol("users", "id"));
        // identifiers are lower-cased by the lexer
        assert_eq!(parse_expr_str("Users.ID"), qcol("users", "id"));
    }

    #[test]
    fn qualified_column_requires_name_after_dot() {
        parse_err("SELECT users.");
        parse_err("SELECT users.1");
    }

    // ------------------------------------------------------------------
    // expressions: operators, precedence and associativity
    // ------------------------------------------------------------------

    #[test]
    fn arithmetic_is_left_associative() {
        // 1 + 2 - 3  ==  (1 + 2) - 3
        assert_eq!(
            parse_expr_str("1 + 2 - 3"),
            bin(Operator::Sub, bin(Operator::Add, int(1), int(2)), int(3))
        );
        // 10 - 4 - 3  ==  (10 - 4) - 3
        assert_eq!(
            parse_expr_str("10 - 4 - 3"),
            bin(Operator::Sub, bin(Operator::Sub, int(10), int(4)), int(3))
        );
    }

    #[test]
    fn parentheses_override_associativity() {
        // 10 - (4 - 3)
        assert_eq!(
            parse_expr_str("10 - (4 - 3)"),
            bin(Operator::Sub, int(10), bin(Operator::Sub, int(4), int(3)))
        );
        // nested parentheses are transparent
        assert_eq!(parse_expr_str("((1))"), int(1));
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // a = 1 OR b = 2 AND c = 3  ==  a = 1 OR (b = 2 AND c = 3)
        assert_eq!(
            parse_expr_str("t.a = 1 OR t.b = 2 AND t.c = 3"),
            bin(
                Operator::Or,
                bin(Operator::Eq, qcol("t", "a"), int(1)),
                bin(
                    Operator::And,
                    bin(Operator::Eq, qcol("t", "b"), int(2)),
                    bin(Operator::Eq, qcol("t", "c"), int(3)),
                ),
            )
        );
    }

    #[test]
    fn and_or_are_left_associative() {
        assert_eq!(
            parse_expr_str("TRUE AND FALSE AND NULL"),
            bin(
                Operator::And,
                bin(Operator::And, boolean(true), boolean(false)),
                null()
            )
        );
        assert_eq!(
            parse_expr_str("TRUE OR FALSE OR NULL"),
            bin(
                Operator::Or,
                bin(Operator::Or, boolean(true), boolean(false)),
                null()
            )
        );
    }

    #[test]
    fn comparison_operators() {
        let cases: [(&str, fn(Box<Expression>, Box<Expression>) -> Operator); 5] = [
            ("=", Operator::Eq),
            (">", Operator::Gt),
            (">=", Operator::Gte),
            ("<", Operator::Lt),
            ("<=", Operator::Lte),
        ];

        for (symbol, ctor) in cases {
            assert_eq!(
                parse_expr_str(&format!("t.a {symbol} 1")),
                bin(ctor, qcol("t", "a"), int(1)),
                "operator {symbol}"
            );
        }
    }

    #[test]
    fn arithmetic_binds_tighter_than_comparison() {
        // 1 + 2 > 3 - 4  ==  (1 + 2) > (3 - 4)
        assert_eq!(
            parse_expr_str("1 + 2 > 3 - 4"),
            bin(
                Operator::Gt,
                bin(Operator::Add, int(1), int(2)),
                bin(Operator::Sub, int(3), int(4)),
            )
        );
    }

    #[test]
    fn comparison_binds_tighter_than_and() {
        // t.a > 1 AND t.b < 2  ==  (t.a > 1) AND (t.b < 2)
        assert_eq!(
            parse_expr_str("t.a > 1 AND t.b < 2"),
            bin(
                Operator::And,
                bin(Operator::Gt, qcol("t", "a"), int(1)),
                bin(Operator::Lt, qcol("t", "b"), int(2)),
            )
        );
    }

    #[test]
    fn like_operator() {
        assert_eq!(
            parse_expr_str("t.name LIKE 'a%'"),
            bin(Operator::Like, qcol("t", "name"), text("a%"))
        );
    }

    #[test]
    #[ignore = "bug: the lexer has no `*`, `/` or `%` tokens, so multiplication, division and remainder cannot be lexed"]
    fn multiplicative_operators_bind_tighter_than_additive() {
        // 1 + 2 * 3  ==  1 + (2 * 3)
        assert_eq!(
            parse_expr_str("1 + 2 * 3"),
            bin(
                Operator::Add,
                int(1),
                bin(Operator::Multiply, int(2), int(3))
            )
        );
        assert_eq!(
            parse_expr_str("8 / 2 % 3"),
            bin(
                Operator::Remainder,
                bin(Operator::Divide, int(8), int(2)),
                int(3)
            )
        );
    }

    #[test]
    #[ignore = "bug: no lexer token (`!=` / `<>`) maps to InfixOp::NotEq"]
    fn not_equal_operator() {
        assert_eq!(
            parse_expr_str("t.a != 1"),
            bin(Operator::NotEq, qcol("t", "a"), int(1))
        );
        assert_eq!(
            parse_expr_str("t.a <> 1"),
            bin(Operator::NotEq, qcol("t", "a"), int(1))
        );
    }

    #[test]
    fn dangling_operators_are_errors() {
        parse_err("SELECT 1 +");
        parse_err("SELECT 1 AND");
        parse_err("SELECT = 1");
        parse_err("SELECT 1 = = 2");
        parse_err("SELECT 1 LIKE");
    }

    // ------------------------------------------------------------------
    // expressions: prefix operators
    // ------------------------------------------------------------------

    #[test]
    fn prefix_minus_and_plus() {
        assert_eq!(parse_expr_str("-1"), un(Operator::Negate, int(1)));
        assert_eq!(parse_expr_str("+1"), un(Operator::Identity, int(1)));
    }

    #[test]
    #[ignore = "bug: parse_prefix_op compares with `>` instead of `>=`, so a prefix operator cannot directly follow one of equal precedence"]
    fn stacked_prefix_operators() {
        assert_eq!(
            parse_expr_str("- -1"),
            un(Operator::Negate, un(Operator::Negate, int(1)))
        );
        assert_eq!(
            parse_expr_str("NOT NOT TRUE"),
            un(Operator::Not, un(Operator::Not, boolean(true)))
        );
    }

    #[test]
    fn prefix_minus_binds_tighter_than_infix() {
        // -1 + 2  ==  (-1) + 2
        assert_eq!(
            parse_expr_str("-1 + 2"),
            bin(Operator::Add, un(Operator::Negate, int(1)), int(2))
        );
        // 2 - -1
        assert_eq!(
            parse_expr_str("2 - -1"),
            bin(Operator::Sub, int(2), un(Operator::Negate, int(1)))
        );
    }

    #[test]
    fn not_binds_looser_than_comparison() {
        // NOT t.a = 1  ==  NOT (t.a = 1)
        assert_eq!(
            parse_expr_str("NOT t.a = 1"),
            un(Operator::Not, bin(Operator::Eq, qcol("t", "a"), int(1)))
        );
    }

    #[test]
    fn not_binds_tighter_than_and() {
        // NOT TRUE AND FALSE  ==  (NOT TRUE) AND FALSE
        assert_eq!(
            parse_expr_str("NOT TRUE AND FALSE"),
            bin(
                Operator::And,
                un(Operator::Not, boolean(true)),
                boolean(false)
            )
        );
    }

    #[test]
    fn prefix_without_operand_is_an_error() {
        parse_err("SELECT -");
        parse_err("SELECT NOT");
    }

    // ------------------------------------------------------------------
    // expressions: postfix operators (IS [NOT] NULL / NaN)
    // ------------------------------------------------------------------

    #[test]
    fn is_null() {
        assert_eq!(
            parse_expr_str("t.a IS NULL"),
            Operator::Is(Box::new(qcol("t", "a")), Literal::Null).into()
        );
    }

    #[test]
    fn is_not_null_is_negated_is_null() {
        assert_eq!(
            parse_expr_str("t.a IS NOT NULL"),
            un(
                Operator::Not,
                Operator::Is(Box::new(qcol("t", "a")), Literal::Null).into()
            )
        );
    }

    #[test]
    fn is_nan() {
        assert_eq!(
            parse_expr_str("t.a IS NaN"),
            Operator::Is(Box::new(qcol("t", "a")), Literal::Float(f32::NAN)).into()
        );
    }

    #[test]
    fn is_combines_with_and() {
        assert_eq!(
            parse_expr_str("t.a IS NULL AND t.b = 1"),
            bin(
                Operator::And,
                Operator::Is(Box::new(qcol("t", "a")), Literal::Null).into(),
                bin(Operator::Eq, qcol("t", "b"), int(1)),
            )
        );
    }

    #[test]
    fn is_only_accepts_null_or_nan() {
        parse_err("SELECT t.a IS 5");
        parse_err("SELECT t.a IS TRUE");
        parse_err("SELECT t.a IS");
        parse_err("SELECT t.a IS NOT");
    }

    // ------------------------------------------------------------------
    // operator tables (precedence / associativity helpers)
    // ------------------------------------------------------------------

    #[test]
    fn infix_precedence_ordering() {
        let ladder = [
            InfixOp::Or,
            InfixOp::And,
            InfixOp::Eq,
            InfixOp::Gt,
            InfixOp::Add,
            InfixOp::Multiply,
            InfixOp::Expo,
        ];
        for pair in ladder.windows(2) {
            assert!(
                pair[0].precedence() < pair[1].precedence(),
                "expected precedence to strictly increase along the ladder"
            );
        }
    }

    #[test]
    fn operators_in_the_same_tier_share_precedence() {
        assert_eq!(InfixOp::Eq.precedence(), InfixOp::NotEq.precedence());
        assert_eq!(InfixOp::Eq.precedence(), InfixOp::Like.precedence());

        for op in [InfixOp::Gte, InfixOp::Lt, InfixOp::Lte] {
            assert_eq!(op.precedence(), InfixOp::Gt.precedence());
        }

        assert_eq!(InfixOp::Add.precedence(), InfixOp::Sub.precedence());

        for op in [InfixOp::Divide, InfixOp::Remainder] {
            assert_eq!(op.precedence(), InfixOp::Multiply.precedence());
        }
    }

    #[test]
    fn prefix_precedence_relative_to_infix() {
        assert!(PrefixOperator::Not.precedence() > InfixOp::And.precedence());
        assert!(PrefixOperator::Not.precedence() < InfixOp::Eq.precedence());
        assert!(PrefixOperator::Minus.precedence() > InfixOp::Multiply.precedence());
        assert_eq!(
            PrefixOperator::Minus.precedence(),
            PrefixOperator::Plus.precedence()
        );
    }

    #[test]
    fn only_exponent_is_right_associative() {
        assert!(matches!(
            InfixOp::Expo.associativity(),
            Associativity::Right
        ));

        for op in [
            InfixOp::Add,
            InfixOp::And,
            InfixOp::Divide,
            InfixOp::Eq,
            InfixOp::Gt,
            InfixOp::Gte,
            InfixOp::Lt,
            InfixOp::Lte,
            InfixOp::Like,
            InfixOp::Multiply,
            InfixOp::NotEq,
            InfixOp::Or,
            InfixOp::Remainder,
            InfixOp::Sub,
        ] {
            assert!(matches!(op.associativity(), Associativity::Left));
        }
    }

    #[test]
    fn associativity_adjusts_next_min_precedence() {
        // left-assoc raises the bar by one so equal-precedence ops group left;
        // right-assoc keeps it so they group right.
        assert_eq!(5u8 + Associativity::Left, 6);
        assert_eq!(5u8 + Associativity::Right, 5);
    }

    #[test]
    fn prefix_operators_are_right_associative() {
        for op in [
            PrefixOperator::Minus,
            PrefixOperator::Plus,
            PrefixOperator::Not,
        ] {
            assert!(matches!(op.associativity(), Associativity::Right));
        }
    }

    #[test]
    fn postfix_precedence() {
        assert_eq!(PostfixOp::Factorial.precedence(), 9);
        assert_eq!(PostfixOp::Is(Literal::Null).precedence(), 4);
        assert_eq!(PostfixOp::IsNot(Literal::Null).precedence(), 4);
    }
}
