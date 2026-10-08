use std::collections::{HashMap, HashSet};

use crate::{
    error::{Error, Result},
    query::{
        parser::ats,
        types::{schema::Table, values::Label},
    },
};

#[derive(Default)]
pub struct Context {
    columns: Vec<Label>,
    tables: HashSet<String>,
    qualified: HashMap<(String, String), usize>,
    unqualified: HashMap<String, Vec<usize>>,
    aggregates: HashMap<ats::Expression, usize>,
    hidden: HashSet<usize>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_table(table: &Table) -> Result<Self> {
        let mut ctx = Self::new();
        ctx.add_table(table, None)?;
        Ok(ctx)
    }

    pub fn add_table(&mut self, table: &Table, alias: Option<&str>) -> Result<()> {
        let name = alias.unwrap_or(&table.name);
        if self.tables.contains(name) {
            return Err(Error::InvalidData(format!("duplicate table name: {name}")));
        }

        for col in &table.column {
            self.add_columns(Label::Qualified(name.to_string(), col.name.clone()));
        }
        self.tables.insert(name.to_string());
        Ok(())
    }

    pub fn add_columns(&mut self, label: Label) -> usize {
        let index = self.columns.len();
        if let Label::Qualified(table, column) = &label {
            self.qualified
                .insert((table.clone(), column.clone()), index);
        }

        if let Label::Qualified(_, name) | Label::Unqualified(name) = &label {
            self.unqualified
                .entry(name.clone())
                .or_default()
                .push(index);
        }

        self.columns.push(label);
        index
    }

    pub fn lookup_column(&self, table: Option<&str>, name: &str) -> Result<usize> {
        let fmtname = || {
            table
                .map(|table| format!("{table}.{name}"))
                .unwrap_or(name.to_string())
        };

        if let Some(table) = table {
            if !self.tables.contains(table) {
                return Err(Error::InvalidInput(format!("unknouns table :{table}")));
            }
            if let Some(index) = self.qualified.get(&(table.to_string(), name.to_string())) {
                return Ok(*index);
            }
        } else if let Some(idx) = self.unqualified.get(name) {
            if idx.len() > 1 {
                return Err(Error::InvalidInput(format!("ambiguous column {name}")));
            }

            return Ok(idx[0]);
        }

        if !self.aggregates.is_empty() {
            return Err(Error::InvalidInput(format!(
                "column {} must be used in an aggregate or GROUP BY expression",
                fmtname()
            )));
        }

        Err(Error::InvalidInput(format!("unknown column {}", fmtname())))
    }

    pub fn add_aggregate(&mut self, expr: &ats::Expression, parent: &Context) -> Option<usize> {
        if self.aggregates.contains_key(expr) {
            return None;
        }

        let mut label = Label::None;
        if let ats::Expression::Column(table, column) = expr {
            if let Ok(index) = parent.lookup_column(table.as_deref(), column.as_str()) {
                label = parent.columns[index].clone();
            }
        }
        let index = self.add_columns(label);
        self.aggregates.insert(expr.clone(), index);
        Some(index)
    }

    pub fn lookup_aggregate(&self, expr: ats::Expression) -> Option<usize> {
        self.aggregates.get(&expr).copied()
    }

    pub fn merge(&mut self, ctx: Context) -> Result<()> {
        for table in ctx.tables {
            if !self.tables.contains(&table) {
                return Err(Error::InvalidInput(format!("duplicate table name {table}")));
            }
            self.tables.insert(table);
        }

        let offset = self.columns.len();
        for label in ctx.columns {
            self.add_columns(label);
        }

        for (expr, index) in ctx.aggregates {
            self.aggregates.entry(expr).or_insert(index + offset);
        }
        self.hidden
            .extend(ctx.hidden.into_iter().map(|index| index + offset));

        Ok(())
    }
}
