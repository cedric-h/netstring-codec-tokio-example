use srvr::NetstringCodec;
use tokio;
use tokio::codec::Framed;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = "127.0.0.1:6142".parse()?;
    let listener = TcpListener::bind(&addr)?;
    let server = listener
        .incoming()
        .for_each(move |socket| {
            process(socket);
            Ok(())
        })
        .map_err(|err| {
            println!("accept error = {:?}", err);
        });

    println!("Running on localhost:6142");
    tokio::run(server);

    Ok(())
}

// Spawn a task to manage the socket.
fn process(socket: TcpStream) {
    // transform our stream of bytes to stream of frames.
    // This is where the magic happens
    let framed_sock = Framed::new(socket, NetstringCodec::new(123, true));

    let connection = Peer::new(framed_sock).map_err(|e| {
        println!("connection error = {:?}", e);
    });
    // spawn the task. Internally, this submits the task to a thread pool
    tokio::spawn(connection);
}

// Struct for each connected clients.
struct Peer {
    socket: Framed<TcpStream, NetstringCodec>,
}

impl Peer {
    fn new(socket: Framed<TcpStream, NetstringCodec>) -> Peer {
        Peer { socket }
    }
}

impl Future for Peer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        while let Async::Ready(line) = self.socket.poll()? {
            match line {
                Some(d) => {
                    dbg!(d);
                }
                // eol/something bad happend in decoding -> disconnect.
                None => return Ok(Async::Ready(())),
            }
        }

        Ok(Async::NotReady)
    }
}

