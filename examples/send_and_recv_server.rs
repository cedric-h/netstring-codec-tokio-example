use netstring_codec_tokio_example::{Message, NetstringCodec};
use tokio;
use tokio::codec::Framed;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = "127.0.0.1:8082".parse()?;
    let listener = TcpListener::bind(&addr)?;
    let server = listener
        .incoming()
        .for_each(move |socket| {
            process(socket);
            Ok(())
        })
        .map_err(|err| {
            println!("accept error = {}", err);
        });

    println!("Running on localhost:8082");
    tokio::run(server);

    Ok(())
}

// Spawn a task to manage the socket.
fn process(socket: TcpStream) {
    // transform our stream of bytes to stream of frames.
    // This is where the magic happens
    let mut framed_sock = Framed::new(socket, NetstringCodec::new(323, true));

    let connection = Peer::new(framed_sock).map_err(|e| {
        println!("connection error = {}", e);
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
                    let msg = Message::unpack(d).unwrap(); // This is a Result
                    self.socket.start_send(
                        Message {
                            x: msg.x.clone(),
                            msg: "The server is responding to your message! \
                                Here's the number you sent!".to_owned(),
                        }
                        .pack()
                        .unwrap(),
                    );
                    self.socket.poll_complete();
                    dbg!(msg);
                }
                // eol/something bad happend in decoding -> disconnect.
                None => return Ok(Async::Ready(())),
            }
        }

        Ok(Async::NotReady)
    }
}
