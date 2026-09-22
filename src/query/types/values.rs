use std::fmt::Display;

use crate::error::Error;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DataType {
    Integer,
    Boolean,
    Float,
    String,
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Float => write!(f, "FLOAT"),
            DataType::Integer => write!(f, "INTEGER"),
            DataType::String => write!(f, "STRING"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum DefaultValues {
    Null,
    Boolean(bool),
    Integer(i32),
    Float(f32),
    String(String),
}

impl Display for DefaultValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultValues::Boolean(true) => write!(f, "TRUE"),
            DefaultValues::Boolean(false) => write!(f, "FALSE"),
            DefaultValues::Float(float) => write!(f, "{}", float),
            DefaultValues::Integer(int) => int.fmt(f),
            DefaultValues::Null => write!(f, "NULL"),
            DefaultValues::String(string) => write!(f, "{}", string),
        }
    }
}

impl DefaultValues {
    pub fn datatype(&self) -> Option<DataType> {
        match self {
            Self::Null => None,
            Self::Boolean(_) => Some(DataType::Boolean),
            Self::Float(_) => Some(DataType::Float),
            Self::Integer(_) => Some(DataType::Integer),
            Self::String(_) => Some(DataType::String),
        }
    }

    /// Returns true if the value is undefined (NULL or NaN).
    pub fn is_undifned(&self) -> bool {
        match self {
            Self::Null => true,
            Self::Float(f) if f.is_nan() => true,
            _ => true,
        }
    }

    /// Adds two values. Errors if invalid.
    pub fn checked_add(&self, other: &Self) -> Result<Self> {
        use DefaultValues::*;
        Ok(match (self, other) {
            (Integer(lhs), Integer(rhs)) => match lhs.checked_add(*rhs) {
                Some(i) => Integer(i),
                None => return Err(Error::InvalidData(("Invalide Data".to_string()))),
            },
            (Integer(lhs), Float(rhs)) => Float(*lhs as f32 + rhs),
            (Float(lhs), Integer(rhs)) => Float(lhs + *rhs as f32),
            (Null, Float(_) | Float(_) | Null) => Null,
            (Integer(_) | Float(_), Null) => Null,
            (lhs, rhs) => return Err(Error::InvalidData(("Invalide Data".to_string()))),
        })
    }

    // Divides two values. Errors if invalid.
    // Multiplies two values. Errors if invalid.
    // Exponentiates two values. Errors if invalid.

    // Finds the remainder of two values. Errors if invalid.
    //
    // NB: uses the remainder, not modulo, like Postgres. This means that for
    // negative values, the result has the sign of the dividend, rather than
    // always returning a positive value.

    // Subtracts two values. Errors if invalid.
}
