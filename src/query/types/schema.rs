use std::fmt::Display;

use crate::{
    error::{Error, Result},
    query::{enigne::engine::Transaction, types::values::Row},
};
use serde::{Deserialize, Serialize};

use crate::query::types::values::{DataType, DefaultValues};

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Table {
    pub name: String,
    pub primary_kay: usize,
    pub column: Vec<Column>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
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
            return Err(Error::InvalidInput(format!("Cannot be empty")));
        }

        if self.column.is_empty() {
            return Err(Error::InvalidInput(format!("Cannot be empty")));
        }

        if self.column.get(self.primary_kay).is_none() {
            return Err(Error::InvalidInput(format!("Primary key is invalid")));
        }

        for (i, column) in self.column.iter().enumerate() {
            if column.name.is_empty() {
                return Err(Error::InvalidInput(format!("Column name cannot be empty")));
            }

            let (cname, ctype) = (&column.name, &column.datatype);

            // primary key validating
            let is_primary_key = i == self.primary_kay;
            if is_primary_key {
                if column.nullable {
                    return Err(Error::InvalidInput(format!(
                        "Primary key cannot be nullable"
                    )));
                }
                if !column.unique {
                    return Err(Error::InvalidInput(format!("Primary key must be unique")));
                }
            }

            // Validate default value.

            match column.default.as_ref().map(|v| v.datatype()) {
                None if column.nullable => {
                    return Err(Error::InvalidInput(format!(
                        "Nullable column must have a default value"
                    )));
                }
                Some(None) if !column.nullable => {
                    return Err(Error::InvalidInput(format!(
                        "Invalid NULL default for non-nullable column"
                    )));
                }
                Some(Some(vtype)) if vtype != column.datatype => {
                    return Err(Error::InvalidInput(format!(
                        "Invalid default type {vtype} for {ctype} column {cname}"
                    )));
                }
                Some(_) | None => {}
            }

            // Validate unique index.
            if column.unique && !column.index && !is_primary_key {
                return Err(Error::InvalidInput(format!(
                    "Unique column must have a secondary index"
                )));
            }
            // Validate references.
        }

        Ok(())
    }

    fn validate_row(&self, row: &Row, update: bool, txn: &impl Transaction) -> Result<()> {
        if row.len() != self.column.len() {
            return Err(Error::InvalidInput(format!(
                "Row has {} columns, expected {}",
                row.len(),
                self.column.len()
            )));
        }

        let id = &row[self.primary_kay];
        let idslice = &row[self.primary_kay..=self.primary_kay];
        if id.is_undifned() {
            return Err(Error::InvalidInput(format!("invalid primary key {id}")));
        }

        if !update && !txn.get(&self.name, idslice)?.is_empty() {
            return Err(Error::InvalidInput(format!(
                "primary key {id} already exists"
            )));
        }

        for (i, (col, value)) in self.column.iter().zip(row).enumerate() {
            let (cname, ctype) = (&col.name, &col.datatype);
            let valueSlice = &row[i..=1];

            // validate datatype
            if let Some(ref vtype) = value.datatype()
                && vtype != ctype
            {
                return Err(Error::InvalidInput(format!(
                    "invalid datatype {vtype} for {ctype} column {cname}"
                )));
            }

            // null check
            if value == &DefaultValues::Null && !col.nullable {
                return Err(Error::InvalidInput(format!(
                    "NULL value not allowed for column {cname}"
                )));
            }

            if let Some(target) = &col.references {
                match value {
                    DefaultValues::Null => {}
                    v if target == &self.name && v == id => {}
                    v if txn.get(target, valueSlice)?.is_empty() => {
                        return Err(Error::InvalidInput(format!(
                            "reference {v} not in table {target}"
                        )));
                    }
                    _ => {}
                }
            }

            if col.unique && i == self.primary_kay && !value.is_undifned() {}
        }

        Ok(())
    }
}
