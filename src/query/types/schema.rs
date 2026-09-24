use std::fmt::Display;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::query::types::values::{DataType, DefaultValues};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Table {
    pub name: String,
    pub primary_kay: usize,
    pub column: Vec<Column>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Column {
    pub name: String,
    pub index: bool,
    pub datatype: DataType,
    pub default: Option<DefaultValues>,
    pub nullable: bool,
    pub references: Option<String>,
    pub unique: bool,
}

impl Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "CREATE TABLE {} (", &self.name)?;
        for (i, column) in self.column.iter().enumerate() {
            if i == self.primary_kay {
                write!(f, " PRIMARY KEY")?;
            } else if !column.nullable {
                write!(f, " NOT NULL")?;
            }

            if let Some(default) = &column.default {
                write!(f, " DEFAULT {default}")?;
            }

            if i != self.primary_kay {
                if column.unique {
                    write!(f, " UNIQUE")?;
                }
                if column.index {
                    write!(f, " INDEX")?;
                }
            }

            if let Some(refence) = &column.references {
                write!(f, " REFERENCES {refence}")?;
            }

            if i < self.column.len() - 1 {
                write!(f, ",")?;
            }

            writeln!(f)?;
        }
        write!(f, ")")
    }
}

impl Table {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            // ERROR: cannot be empty
        }

        if self.column.is_empty() {
            // ERROR: column cant tbe empty
        }

        if self.column.get(self.primary_kay).is_none() {
            // ERROR: primamry cant be invalid
        }

        for (i, column) in self.column.iter().enumerate() {
            if column.name.is_empty() {
                //ERROR: column name cant be empty
            }

            // primary key validating
            let is_primary_key = i == self.primary_kay;
            if is_primary_key {
                if column.nullable {
                    // ERROR primary key cant be nullanle
                }
                if !column.unique {
                    // ERROR primary key must be unique
                }
            }

            // Validate default value.

            match column.default.as_ref().map(|v| v.datatype()) {
                None if column.nullable => {
                    // ERROR: "nullable column {cname} must have a default value
                }
                Some(None) if !column.nullable => {
                    // ERROR: invalid NULL default for non-nullable column {cname}
                }
                Some(Some(vtype)) if vtype != column.datatype => {
                    // ERROR : invalid default type {vtype} for {ctype} column {cname}
                }
                Some(_) | None => {}
            }

            // Validate unique index.
            if column.unique && !column.index && !is_primary_key {
                // ERROR: unique column {cname} must have a secondary index
            }
            // Validate references.
        }

        Ok(())
    }
}
