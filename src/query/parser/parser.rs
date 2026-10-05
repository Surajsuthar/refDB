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

        let mut set = BTreeMap::new();
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

            Token::Number(n) if n.chars().all(|c| c.is_ascii_digit()) => {
                ats::Literal::Integer(n.parse()?).into()
            }
            Token::Number(n) => ats::Literal::Float(n.parse()?).into(),
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

        Ok(self.next_if(|t| {
            match *t {
                Token::Ex
            }
        }))
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

    #[test]
    fn parse_begin() {
        let stmt = Parser::parse("BEGIN;").unwrap();
    }
}
