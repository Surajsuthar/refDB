use std::collections::HashMap;

use crate::query::{parser::ats::Expression, types::schema::Table};

pub enum Plan {
    CreateTable {
        schema: Table,
    },
    DropTable {
        name: String,
    },
    Delete {
        table: String,
        primary_key: usize,
    },
    Insert {
        table: String,
        column: Option<HashMap<usize, usize>>,
    },
    Update {
        table: String,
        primary_key: usize,
        expressions: Vec<(usize, Expression)>,
    },
}
