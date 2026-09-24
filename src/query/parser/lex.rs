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

        // self.chars.next();
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

mod test {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        Lexer::new(input).collect::<Result<Vec<_>>>().unwrap()
    }

    #[test]
    fn text_lex_select() {
        let token = lex("SELECT name FROM users");
        assert_eq!(
            token,
            vec![
                Token::Keyword(Keyword::Select),
                Token::Identifer("name".into()),
                Token::Keyword(Keyword::From),
                Token::Identifer("users".into())
            ]
        )
    }

    #[test]
    fn text_lex_keyword() {
        let tokens = lex("SELECT FROM WHERE LIMIT OFFSET AND OR BY ASC DESC
                 INSERT INTO VALUES UPDATE DELETE DROP TABLE AS");

        assert_eq!(
            tokens,
            vec![
                Token::Keyword(Keyword::Select),
                Token::Keyword(Keyword::From),
                Token::Keyword(Keyword::Where),
                Token::Keyword(Keyword::Limit),
                Token::Keyword(Keyword::Offset),
                Token::Keyword(Keyword::And),
                Token::Keyword(Keyword::Or),
                Token::Keyword(Keyword::By),
                Token::Keyword(Keyword::Asc),
                Token::Keyword(Keyword::Desc),
                Token::Keyword(Keyword::Insert),
                Token::Keyword(Keyword::Into),
                Token::Keyword(Keyword::Values),
                Token::Keyword(Keyword::Update),
                Token::Keyword(Keyword::Delete),
                Token::Keyword(Keyword::Drop),
                Token::Keyword(Keyword::Table),
                Token::Keyword(Keyword::As),
            ]
        );
    }

    #[test]
    fn test_lex_identifiers() {
        let tokens = lex("users user_id first_name");
        assert_eq!(
            tokens,
            vec![
                Token::Identifer("users".into()),
                Token::Identifer("user_id".into()),
                Token::Identifer("first_name".into()),
            ]
        );
    }

    #[test]
    fn test_lex_case_insensitive() {
        let tokens = lex("SELECT Select select SeLeCt");

        assert_eq!(
            tokens,
            vec![
                Token::Keyword(Keyword::Select),
                Token::Keyword(Keyword::Select),
                Token::Keyword(Keyword::Select),
                Token::Keyword(Keyword::Select),
            ]
        );
    }

    #[test]
    fn test_lex_identifier_case() {
        let tokens = lex("Users USER_NAME");

        assert_eq!(
            tokens,
            vec![
                Token::Identifer("users".into()),
                Token::Identifer("user_name".into()),
            ]
        );
    }

    #[test]
    fn test_lex_numbers() {
        let tokens = lex("123 456 3.14 0.5");

        assert_eq!(
            tokens,
            vec![
                Token::Number("123".into()),
                Token::Number("456".into()),
                Token::Number("3.14".into()),
                Token::Number("0.5".into()),
            ]
        );
    }

    #[test]
    fn test_lex_decimal_edge_cases() {
        let tokens = lex("1. 1.5 0.25");

        assert_eq!(
            tokens,
            vec![
                Token::Number("1.".into()),
                Token::Number("1.5".into()),
                Token::Number("0.25".into()),
            ]
        );
    }

    #[test]
    fn test_lex_strings() {
        let tokens = lex("'hello' 'hello world' '123'");

        assert_eq!(
            tokens,
            vec![
                Token::String("hello".into()),
                Token::String("hello world".into()),
                Token::String("123".into()),
            ]
        );
    }

    #[test]
    fn test_lex_escaped_string() {
        let tokens = lex("'hello ''world'''");
        assert_eq!(tokens, vec![Token::String("hello 'world'".into()),]);
    }

    #[test]
    fn test_lex_quoted_identifier() {
        let tokens = lex(r#""user name" "select" "hello""world""#);

        assert_eq!(
            tokens,
            vec![
                Token::Identifer("user name".into()),
                Token::Identifer("select".into()),
                Token::Identifer("hello\"world".into()),
            ]
        );
    }

    #[test]
    fn test_lex_real_query() {
        let tokens = lex("SELECT id, name
             FROM users
             WHERE age >= 18
             ORDER BY name ASC
             LIMIT 10;");

        assert_eq!(
            tokens,
            vec![
                Token::Keyword(Keyword::Select),
                Token::Identifer("id".into()),
                Token::Comma,
                Token::Identifer("name".into()),
                Token::Keyword(Keyword::From),
                Token::Identifer("users".into()),
                Token::Keyword(Keyword::Where),
                Token::Identifer("age".into()),
                Token::Gte,
                Token::Number("18".into()),
                Token::Keyword(Keyword::Order),
                Token::Keyword(Keyword::By),
                Token::Identifer("name".into()),
                Token::Keyword(Keyword::Asc),
                Token::Keyword(Keyword::Limit),
                Token::Number("10".into()),
                Token::SemiColon,
            ]
        );
    }
}
