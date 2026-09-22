use std::{fmt::format, iter::Peekable};

use crate::{
    error::Result,
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

    fn next_if(&mut self, fun: impl Fn(&Token) -> bool) -> Option<Token> {
        self.peek().ok()?.filter(|t| fun(t))?;
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
            Token::Keyword(Keyword::Create) => self.parse_create_table(),
            Token::Keyword(Keyword::Select) => self.parse_select(),
            Token::Keyword(Keyword::Insert) => self.parse_insert(),

            token => Err(Error::InvalidInput(format!("unexpected token"))),
        }
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

    fn parse_select_clause(&mut self) -> Result<Vec<(Expression, String)>> {
        unimplemented!()
    }

    fn parse_from_clause(&mut self) -> Result<Vec<From>> {
        unimplemented!()
    }

    fn parse_where_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Where.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_group_by_clause(&mut self) -> Result<Option<Expression>> {
        unimplemented!()
    }

    fn parse_having_clause(&mut self) -> Result<Option<Expression>> {
        if self.next_if(|c| *c == Keyword::Having.into()).is_none() {
            return Ok(None);
        }
        Ok(Some(self.parse_expr()?))
    }

    fn parse_order_by_clause(&mut self) -> Result<Option<Expression>> {
        unimplemented!()
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

    fn parse_expr(&mut self) -> Result<Expression> {
        unimplemented!()
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
        unimplemented!()
    }
}
