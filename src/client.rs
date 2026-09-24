use std::{
    io::{BufReader, BufWriter, Write},
    net::{TcpStream, ToSocketAddrs},
};

use crate::{
    error::{Error, Result},
    query::execution::session::StmtResult,
    server::{Request, Response},
};

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl Client {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let socket = TcpStream::connect(addr).map_err(|e| Error::IO(e.to_string()))?;
        let reader = BufReader::new(socket.try_clone().map_err(|e| Error::IO(e.to_string()))?);
        let writer = BufWriter::new(socket);

        Ok(Client {
            reader: reader,
            writer,
        })
    }

    fn request(&mut self, req: Request) -> Result<Response> {
        // encode request
        self.writer.flush();
        // decode request
        unimplemented!();
    }

    pub fn execute(&mut self, stmt: &str) -> Result<StmtResult> {
        let result = match self.request(Request::Execute(stmt.to_string()))? {
            Response::Execute(result) => result,
            response => {
                return Err(Error::InvalidData(format!(
                    "unexpected response {response:?}"
                )));
            }
        };

        match &result {
            _ => {}
        }

        Ok(result)
    }

    pub fn get_table(&mut self, table: &str) -> Result<StmtResult> {
        unimplemented!();
    }

    pub fn list_table(&mut self) -> Result<Vec<String>> {
        unimplemented!();
    }
}
