use crate::error::{Error, Result};
use std::{fmt::Display, iter::Peekable, str::Chars};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    Keyword(Keyword),
    String(String),
    Identifer(String),

    Eq,
    Gt,
    Gte,
    Lt,
    Lte,

    Plus,
    Minus,

    OpenParen,
    CloseParen,
    SemiColon,
    Comma,
    Colon,
    Period,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Keyword {
    Select,
    Create,
    From,
    Where,
    Limit,
    Offset,
    And,
    Or,
    By,
    Asc,
    Desc,
    Insert,
    Into,
    Values,
    Update,
    Delete,
    Drop,
    Table,
    As,
    Primary,
    Null,
    Not,
    Default,
    Index,
    Reference,
    Unique,
    Having,
    Begin,
    Commit,
    Rollback,
    Explain,
    Order,
    Join,
    Left,
    Right,
    Inner,
    Cross,

    Bool,
    Boolean,
    String,
    Text,
    Varchar,
    Int,
    Integer,
    Float,
    Double,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::And => "AND",
            Self::Asc => "ASC",
            Self::By => "BY",
            Self::Create => "CREATE",
            Self::Desc => "DESC",
            Self::From => "FROM",
            Self::Limit => "LIMIT",
            Self::Offset => "OFFSET",
            Self::Or => "OR",
            Self::Select => "SELECT",
            Self::Where => "WHERE",
            Self::Table => "TABLE",
            Self::Update => "UPDATE",
            Self::Values => "VALUES",
            Self::Delete => "DELETE",
            Self::Into => "INTO",
            Self::Unique => "UNIQUE",
            Self::Insert => "INSERT",
            Self::As => "AS",
            Self::Primary => "PRIMARY",
            Self::Null => "NULL",
            Self::Not => "NOT",
            Self::Default => "DEFAULT",
            Self::Index => "INDEX",
            Self::Reference => "REFERENCE",
            Self::Bool => "BOOL",
            Self::Boolean => "BOOLEAN",
            Self::String => "STRING",
            Self::Text => "TEXT",
            Self::Varchar => "VARCHAR",
            Self::Begin => "BEGIN",
            Self::Commit => "COMMIT",
            Self::Order => "ORDER",
            Self::Rollback => "ROLLBACK",
            Self::Explain => "EXPLAIN",
            Self::Having => "HAVING",
            Self::Join => "JOIN",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
            Self::Inner => "INNER",
            Self::Cross => "CROSS",

            Self::Int => "INT",
            Self::Drop => "DROP",
            Self::Integer => "INTEGER",
            Self::Float => "FLOAT",
            Self::Double => "DOUBLE",
        })
    }
}

impl TryFrom<&str> for Keyword {
    type Error = &'static str;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        debug_assert!(value.chars().all(|c| !c.is_uppercase()));
        Ok(match value {
            "and" => Self::And,
            "asc" => Self::Asc,
            "by" => Self::By,
            "create" => Self::Create,
            "insert" => Self::Insert,
            "desc" => Self::Desc,
            "from" => Self::From,
            "limit" => Self::Limit,
            "offset" => Self::Offset,
            "into" => Self::Into,
            "update" => Self::Update,
            "delete" => Self::Delete,
            "table" => Self::Table,
            "as" => Self::As,
            _ => return Err("unknown keyword"),
        })
    }
}

impl From<Keyword> for Token {
    fn from(value: Keyword) -> Self {
        Self::Keyword(value)
    }
}

#[derive(Debug)]
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Result<Token>> {
        match self.scan() {
            Ok(Some(token)) => Some(Ok(token)),
            Ok(None) => self
                .chars
                .peek()
                .map(|c| Err(Error::InvalidInput(format!("unexpected character: {}", c)))),
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Lexer {
        Lexer {
            chars: input.chars().peekable(),
        }
    }

    fn scan(&mut self) -> Result<Option<Token>> {
        self.skip_whitespcae();

        let Some(c) = self.chars.peek() else {
            return Ok(None);
        };

        match c {
            '\'' => self.scan_string(),
            '"' => Ok(self.scan_identifier_quoted()),
            '0'..='9' => Ok(self.scan_number()),
            c if c.is_alphabetic() => Ok(self.scan_ident_or_keyword()),
            _ => Ok(self.scan_symbol()),
        }
    }

    fn next_if(&mut self, predicate: impl Fn(char) -> bool) -> Option<char> {
        self.chars.peek().filter(|&&c| predicate(c))?;
        self.chars.next()
    }

    fn scan_symbol(&mut self) -> Option<Token> {
        let token = self.chars.next().and_then(|c| match c {
            '.' => Some(Token::Period),
            '=' => Some(Token::Eq),
            '>' => Some(Token::Gt),
            '<' => Some(Token::Lt),
            '+' => Some(Token::Plus),
            '-' => Some(Token::Minus),
            ',' => Some(Token::Comma),
            ';' => Some(Token::SemiColon),
            '(' => Some(Token::OpenParen),
            ')' => Some(Token::CloseParen),
            _ => None,
        })?;

        let tokne = match token {
            Token::Gt if self.next_if(|c| c == '=').is_some() => Some(Token::Gte),
            Token::Lt if self.next_if(|c| c == '=').is_some() => Some(Token::Lte),
            token => Some(token),
        };

        self.chars.next();
        tokne
    }

    fn scan_ident_or_keyword(&mut self) -> Option<Token> {
        let mut name = self
            .chars
            .next_if(|c| c.is_alphabetic())?
            .to_lowercase()
            .to_string();

        while let Some(c) = self.next_if(|c| c.is_alphanumeric() || c == '_') {
            name.extend(c.to_lowercase())
        }

        if let Ok(keyword) = Keyword::try_from(name.as_str()) {
            return Some(Token::Keyword(keyword));
        }

        Some(Token::Identifer(name))
    }

    fn scan_identifier_quoted(&mut self) -> Option<Token> {
        if self.next_if(|c| c == '"').is_none() {
            return None;
        }

        let mut ident = String::new();
        loop {
            match self.chars.next() {
                Some('"') if self.next_if(|c| c == '"').is_some() => ident.push('"'),
                Some('"') => break,
                Some(c) => ident.push(c),
                None => return None,
            }
        }
        Some(Token::Identifer(ident))
    }

    fn scan_string(&mut self) -> Result<Option<Token>> {
        // Must start with '
        if self.next_if(|c| c == '\'').is_none() {
            return Ok(None);
        }
        let mut string = String::new();
        loop {
            match self.chars.next() {
                // SQL escaped quote: ''
                Some('\'') if self.next_if(|c| c == '\'').is_some() => {
                    string.push('\'');
                }
                // Closing quote
                Some('\'') => break,
                // Normal character
                Some(c) => string.push(c),
                // EOF before closing quote
                None => {
                    return Err(Error::InvalidInput(format!("Invalid Token")));
                }
            }
        }
        Ok(Some(Token::String(string)))
    }

    fn scan_number(&mut self) -> Option<Token> {
        let mut number = self.next_if(|c| c.is_ascii_digit())?.to_string();

        while let Some(c) = self.next_if(|c| c.is_ascii_digit()) {
            number.push(c);
        }

        if self.next_if(|c| c == '.').is_some() {
            number.push('.');
            while let Some(c) = self.next_if(|c| c.is_ascii_digit()) {
                number.push(c);
            }
        }

        // WIP: exponent check
        Some(Token::Number(number))
    }

    fn skip_whitespcae(&mut self) {
        while self.chars.peek().is_some_and(|c| c.is_whitespace()) {
            self.chars.next();
        }
    }
}
