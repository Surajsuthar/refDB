use std::path::PathBuf;

use refDB::query::execution::session::StmtResult;
use refDB::{client::Client, error::Error};
use rustyline::{
    Editor,
    validate::{ValidationContext, ValidationResult},
};

struct Command {
    stmt: Option<String>,

    #[arg(default_value = "localhost")]
    host: String,

    #[arg(short = 'p', long, default_value = "8080")]
    port: u16,
}

impl Command {
    fn run(self) -> Result<()> {
        unimplemented!();
    }
}

struct Shell {
    client: Client,
    editor: Editor,
    history_path: Option<PathBuf>,
    show_header: bool,
}

impl Shell {
    fn new(host: &str, port: u16) -> Result<Self> {
        let client = Client::connect((host, port)?);
        let mut editor = Editor::new()?;

        unimplemented!();
    }

    fn execute(&mut self, input: &str) -> Result<()> {
        if input.start('#') {
            self.ex_commnad(input)
        } else {
            self.ex_sql(input)?
        }
        Ok(())
    }

    fn ex_command(&self, input: &str) -> Result<()> {
        let mut input = input.split_ascii_whitespace();
        let Some(cmd) = input.next() else {
            return Err(Error::InvalidInput(format!("wrong input")));
        };

        let args = input.collect::<Vec>();

        // more cammnd will come in handy
        match (cmd, args.as_slice()) {
            ("#help", []) => println!(),
            (cmd, _) => return Err(Error::InvalidInput(format!("unknown {cmd}"))),
        }

        unimplemented!();
    }

    fn ex_sql(&self, input: &str) -> Result<()> {
        unimplemented!();
    }
}

struct InputValidate;

impl InputValidate {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        let cmd = ctx.input();

        if cmd.is_empty() || cmd.starts_with('#') || cmd == ';' {
            return Ok(ValidationResult::Valid(None));
        }

        Ok(ValidationResult::Incomplete)
    }

    fn validate_while_typing(&self) -> bool {
        false // only check after completed lines
    }
}
