use bytes::Bytes;
use futures::sync::mpsc as futmpsc;
use netstring_codec_tokio_example::{Message, NetstringCodec, SharedDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc as stdmpsc;
use std::thread;
use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::prelude::*;

/// Connect to the remote server and returns interfaces to send and receive
/// messages
///
/// As opposed to the server, we don't need to attach the SocketAddr that
/// sent the message and we don't need to specify the SocketAddr when sending
/// because this is known at start time.
pub fn start_connecting(
    server_addr: SocketAddr,
) -> Result<(SharedDeque<Message>, stdmpsc::Sender<Message>), Box<std::error::Error>>
{
    println!("Start connecting to {}", server_addr);
    // interfaces
    let net_to_game = SharedDeque::new(1024);
    let mut net_to_game_clone = net_to_game.clone();
    let (int_tx, int_rx) = futmpsc::channel(1024);
    let (tx, rx) = stdmpsc::channel();
    let int_rx = int_rx.map_err(|_| panic!("Error not possible on rx"));

    thread::spawn(move || read_channel(int_tx, rx));

    let async_stuff = connect(server_addr, Box::new(int_rx))?;
    thread::spawn(move || {
        tokio::run(
            async_stuff
                .for_each(move |buf| {
                    match Message::unpack(buf.to_vec()) {
                        Ok(unpacked) => {
                            net_to_game_clone.push(unpacked);
                        }
                        Err(e) => {
                            eprintln!(
                                "Received malformed message from {}, error = {}",
                                server_addr, e
                            );
                        }
                    }

                    Ok(())
                })
                .map_err(|e| eprintln!("{}", e)),
        );
    });

    Ok((net_to_game, tx))
}

/// Will create the futures that will run in tokio runtime.
fn connect(
    server_addr: SocketAddr,
    game_to_net: Box<Stream<Item = Bytes, Error = io::Error> + Send>,
) -> Result<
    Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>,
    Box<std::error::Error>,
> {
    let tcp = TcpStream::connect(&server_addr);

    let stream = Box::new(
        tcp.map(move |stream| {
            let (sink, stream) =
                Framed::new(stream, NetstringCodec::new(500, false)).split();

            tokio::spawn(
                game_to_net
                    .map(move |chunk| chunk.to_vec())
                    .forward(sink)
                    .then(|result| {
                        if let Err(e) = result {
                            eprintln!("failed to write to socket: {}", e)
                        }
                        Ok(())
                    }),
            );

            stream
        })
        .flatten_stream(),
    );

    Ok(stream)
}

fn read_channel(mut tx: futmpsc::Sender<Bytes>, rx: stdmpsc::Receiver<Message>) {
    loop {
        match rx.recv() {
            Ok(d) => {
                // if cannot serialize here, we have a problem...
                let packed = d
                    .pack()
                    .map_err(|e| {
                        eprintln!("Error when unpacking in `read_channel` = {}", e);
                        e
                    })
                    .unwrap();

                tx = match tx.send(packed.into()).wait() {
                    Ok(tx) => tx,
                    Err(e) => {
                        eprintln!("Error in read_channel = {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error on read channel = {}", e);
                break;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut from_server, to_server) =
        start_connecting("127.0.0.1:8082".parse().unwrap())?;

    // initialize random generation
    use rand::prelude::*;
    let mut rng = rand::thread_rng();

    loop {
        //compose message to send to the server
        let random_message = Message {
            x: rng.gen(),
            msg: String::from("Hello there!"),
        };

        //emitting
        println!("Sending a message to the server!\n");
        if let Err(e) = to_server.send(random_message) {
            eprintln!("data transmission error: {}", e);
        }

        //recieving
        for event in from_server.drain() {
            println!("Sweet, we got a response from thse server!");
            dbg!(event);
        }

        thread::sleep(std::time::Duration::from_secs(1))
    }

    Ok(())
}
