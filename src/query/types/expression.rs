use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    query::types::values::{DefaultValues, Row},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Constant(DefaultValues),

    Columns(usize),

    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),

    Eq(Box<Expression>, Box<Expression>),
    Gt(Box<Expression>, Box<Expression>),
    Lt(Box<Expression>, Box<Expression>),
    Is(Box<Expression>, DefaultValues),

    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Neglate(Box<Expression>),

    Like(Box<Expression>, Box<Expression>),
}

impl Expression {
    fn evaluate(&self, row: Option<&Row>) -> Result<DefaultValues> {
        use DefaultValues::*;

        Ok(match self {
            Self::Constant(val) => val.clone(),

            Self::Columns(idx) => row
                .and_then(|r| r.get(*idx))
                .cloned()
                .expect("invalid Index"),

            Self::And(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (Boolean(lhs), Boolean(rhs)) => Boolean(lhs && rhs),
                (Boolean(b), Null) | (Null, Boolean(b)) if !b => Boolean(false),
                (Boolean(_), Null) | (Null, Boolean(_)) | (Null, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't AND {lhs} and {rhs}")));
                }
            },

            Self::Or(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (Boolean(lhs), Boolean(rhs)) => Boolean(lhs || rhs),
                (Boolean(b), Null) | (Null, Boolean(b)) if !b => Boolean(true),
                (Boolean(_), Null) | (Null, Boolean(_)) | (Null, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't AND {lhs} and {rhs}")));
                }
            },

            Self::Not(exp) => match exp.evaluate(row)? {
                Boolean(val) => Boolean(val),
                Null => Null,
                value => return Err(Error::InvalidInput(format!("can't NOT {value}"))),
            },

            Self::Eq(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (Integer(lhs), Float(rhs)) => Boolean(lhs as f32 == rhs),
                (Integer(lhs), Integer(rhs)) => Boolean(lhs == rhs),
                (Boolean(lhs), Boolean(rhs)) => Boolean(lhs == rhs),
                (Float(lhs), Float(rhs)) => Boolean(lhs == rhs),
                (Float(lhs), Integer(rhs)) => Boolean(lhs == rhs as f32),
                (String(lhs), String(rhs)) => Boolean(lhs == rhs),
                (Null, _) | (_, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't AND {lhs} and {rhs}")));
                }
            },

            Self::Gt(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (Integer(lhs), Float(rhs)) => Boolean(lhs as f32 > rhs),
                (Integer(lhs), Integer(rhs)) => Boolean(lhs > rhs),
                (Boolean(lhs), Boolean(rhs)) => Boolean(lhs > rhs),
                (Float(lhs), Float(rhs)) => Boolean(lhs > rhs),
                (Float(lhs), Integer(rhs)) => Boolean(lhs > rhs as f32),
                (String(lhs), String(rhs)) => Boolean(lhs > rhs),
                (Null, _) | (_, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't AND {lhs} and {rhs}")));
                }
            },

            Self::Lt(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (Integer(lhs), Float(rhs)) => Boolean((lhs as f32) < rhs),
                (Integer(lhs), Integer(rhs)) => Boolean(lhs < rhs),
                (Boolean(lhs), Boolean(rhs)) => Boolean(lhs < rhs),
                (Float(lhs), Float(rhs)) => Boolean(lhs < rhs),
                (Float(lhs), Integer(rhs)) => Boolean(lhs < rhs as f32),
                (String(lhs), String(rhs)) => Boolean(lhs < rhs),
                (Null, _) | (_, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't AND {lhs} and {rhs}")));
                }
            },

            Self::Is(expr, Null) => Boolean(expr.evaluate(row)? == Null),
            Self::Is(expr, Float(f)) if f.is_nan() => match expr.evaluate(row)? {
                Float(f) => Boolean(f.is_nan()),
                Null => Null,
                val => {
                    return Err(Error::InvalidInput(format!(
                        "IS NAN can't be used with {val}"
                    )));
                }
            },
            Self::Is(_, v) => panic!("invalid IS value {v}"),

            Self::Add(lhs, rhs) => lhs.evaluate(row)?.checked_add(&rhs.evaluate(row)?)?,

            Self::Multiply(lhs, rhs) => lhs.evaluate(row)?.checked_mul(&rhs.evaluate(row)?)?,

            Self::Divide(lhs, rhs) => lhs.evaluate(row)?.checked_div(&rhs.evaluate(row)?)?,

            Self::Subtract(lhs, rhs) => lhs.evaluate(row)?.checked_sub(&rhs.evaluate(row)?)?,

            Self::Neglate(expr) => match expr.evaluate(row)? {
                Integer(i) => Integer(-i),
                Float(f) => Float(-f),
                Boolean(b) => Boolean(!b),
                Null => Null,
                val => return Err(Error::InvalidInput(format!("can't negate {val}"))),
            },

            Self::Like(lhs, rhs) => match (lhs.evaluate(row)?, rhs.evaluate(row)?) {
                (String(lhs), String(rhs)) => {
                    let pattern = format!(
                        "^{}$",
                        regex::escape(&rhs).replace('%', ".*").replace('_', ".")
                    );
                    Boolean(Regex::new(&pattern).map(|r| r.is_match(&lhs)).is_ok())
                }
                (String(_), Null) | (Null, String(_)) | (Null, Null) => Null,
                (lhs, rhs) => {
                    return Err(Error::InvalidInput(format!("can't LIKE {lhs} and {rhs}")));
                }
            },
        })
    }

    fn walk(&mut self, visitor: &mut impl FnMut(&Expression) -> bool) -> bool {
        if !visitor(self) {
            return false;
        }

        match self {
            Self::Add(lhs, rhs)
            | Self::And(lhs, rhs)
            | Self::Divide(lhs, rhs)
            | Self::Or(lhs, rhs)
            | Self::Multiply(lhs, rhs)
            | Self::Like(lhs, rhs)
            | Self::Eq(lhs, rhs)
            | Self::Gt(lhs, rhs)
            | Self::Lt(lhs, rhs)
            | Self::Subtract(lhs, rhs) => lhs.walk(visitor) && rhs.walk(visitor),

            Self::Is(expr, _) | Self::Neglate(expr) | Self::Not(expr) => expr.walk(visitor),

            Self::Constant(_) | Self::Columns(_) => true,
        }
    }

    pub fn contains(&self, visitor: &impl Fn(&Expression) -> bool) -> bool {
        !self.walk(&mut |e| !visitor(e))
    }

    pub fn transform(
        mut self,
        before: &impl Fn(Self) -> Result<Self>,
        after: &impl Fn(Self) -> Result<Self>,
    ) -> Result<Self> {
        let xform = |mut expr: Box<Expression>| -> Result<Box<Expression>> {
            *expr = expr.transform(before, after)?;
            Ok(expr)
        };

        self = before(self)?;
        self = match self {
            Self::Add(lhs, rhs) => Self::Add(xform(lhs)?, xform(rhs)?),
            Self::And(lhs, rhs) => Self::And(xform(lhs)?, xform(rhs)?),
            Self::Or(lhs, rhs) => Self::Or(xform(lhs)?, xform(rhs)?),
            Self::Like(lhs, rhs) => Self::Like(xform(lhs)?, xform(rhs)?),
            Self::Eq(lhs, rhs) => Self::Eq(xform(lhs)?, xform(rhs)?),
            Self::Gt(lhs, rhs) => Self::Gt(xform(lhs)?, xform(rhs)?),
            Self::Lt(lhs, rhs) => Self::Lt(xform(lhs)?, xform(rhs)?),
            Self::Subtract(lhs, rhs) => Self::Subtract(xform(lhs)?, xform(rhs)?),
            Self::Is(lhs, value) => Self::Is(xform(lhs)?, value),
            Self::Neglate(lhs) => Self::Neglate(xform(lhs)?),
            Self::Not(lhs) => Self::Not(xform(lhs)?),

            Self::Divide(lhs, rhs) => Self::Divide(xform(lhs)?, xform(rhs)?),
            Self::Multiply(lhs, rhs) => Self::Multiply(xform(lhs)?, xform(rhs)?),

            expr @ (Self::Constant(_) | Self::Columns(_)) => expr,
        };

        self = after(self)?;
        Ok(self)
    }
}
