use std::{
    io::{BufReader, BufWriter, Result},
    net::{TcpStream, ToSocketAddrs},
};

pub struct Client {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
}

impl Client {
    pub fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let socket = TcpStream::connect(addr)?;
        let reader = BufReader::new(socket.try_clone()?);
        let writer = BufWriter::new(socket);
        Ok(Client {
            reader: reader,
            writer,
        })
    }
}
