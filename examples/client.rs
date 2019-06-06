use netstring_codec_tokio_example::Message;
use std::env;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::thread;

use futures::sync::mpsc;
use tokio::prelude::*;

fn main() -> Result<(), Box<std::error::Error>> {
    let mut args = env::args();
    // Parse what address we're going to connect to
    let addr = match args.nth(1) {
        Some(addr) => addr,
        None => Err("this program requires at least one argument")?,
    };

    let addr = addr.parse::<SocketAddr>()?;
    let (stdin_tx, stdin_rx) = mpsc::channel(0);

    thread::spawn(|| read_stdin(stdin_tx));
    let stdin_rx = stdin_rx.map_err(|_| panic!("errors not possible on rx"));

    let stdout = tcp::connect(&addr, Box::new(stdin_rx))?;
    let mut out = io::stdout();

    tokio::run({
        stdout
            .for_each(move |chunk| out.write_all(&chunk))
            .map_err(|e| println!("error reading stdout; error = {:?}", e))
    });

    Ok(())
}

mod tcp {
    use netstring_codec_tokio_example::{Message, NetstringCodec};

    use tokio;
    use tokio::codec::{Decoder, Framed};
    use tokio::net::TcpStream;
    use tokio::prelude::*;

    //use bytes::BytesMut;
    use std::error::Error;
    use std::io;
    use std::net::SocketAddr;

    pub fn connect(
        addr: &SocketAddr,
        stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>,
    ) -> Result<Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>, Box<Error>>
    {
        let tcp = TcpStream::connect(addr);

        let stream = Box::new(
            tcp.map(move |stream| {
                // magiiic
                let (sink, stream) =
                    Framed::new(stream, NetstringCodec::new(255, false)).split();

                tokio::spawn(stdin.forward(sink).then(|result| {
                    if let Err(e) = result {
                        println!("failed to write to socket: {}", e)
                    }
                    Ok(())
                }));

                stream
            })
            .flatten_stream(),
        );

        Ok(stream)
    }

}

fn read_stdin(mut tx: mpsc::Sender<Vec<u8>>) {
    let mut stdin = io::stdin();

    loop {
        //get input
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);

        let mut input = Message {
            x: 42.0,
            msg: String::from_utf8(buf).unwrap(),
        };

        tx = match tx.send(input.pack()).wait() {
            Ok(tx) => tx,
            Err(_) => break,
        };
    }
}
