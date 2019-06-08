use tokio;
use bytes::{BytesMut};
use tokio::codec::{Decoder, Encoder};
use tokio::io;

mod message;
pub use message::*;
mod shared_dequeue;
pub use shared_dequeue::*;

/// Netstring is an easy way to frame data on TCP.
/// http://cr.yp.to/proto/netstrings.txt
pub struct NetstringCodec {
    state: ParserState,

    current_length: usize,

    /// Max length for the string. This is to avoid attacks by sending
    /// packets that are too large.
    max_length: usize,

    /// Will disconnect the peer on error if this is true.
    disconnect_on_error: bool,
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Length,
    Data,
}

impl Encoder for NetstringCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> io::Result<()> {
        let item_len = item.len().to_string();
        if item.len() >= self.max_length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "data is too large ({}) to send. max length: {}",
                    item_len, self.max_length
                ),
            ));
        }

        let len_string = item_len.as_bytes();
        dst.extend_from_slice(len_string);
        dst.extend_from_slice(":".to_string().as_bytes());
        dst.extend_from_slice(&item[..]);
        dst.extend_from_slice(",".to_string().as_bytes());

        Ok(())
    }
}


impl Decoder for NetstringCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Vec<u8>>, io::Error> {
        self.parse_length(buf)
    }
}

impl NetstringCodec {
    pub fn new(max_length: usize, disconnect_on_error: bool) -> Self {
        Self {
            max_length,
            disconnect_on_error,
            current_length: 0,
            state: ParserState::Length,
        }
    }

    fn parse_length(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<Vec<u8>>, io::Error> {
        // Try to find the current length.
        if self.state == ParserState::Length {
            if let Some(colon_offset) = buf.iter().position(|b| *b == b':') {
                // try to extract the length here.
                let length = buf.split_to(colon_offset + 1);
                let length = &length[..length.len() - 1]; // remove colon from 
                
                //TODO better - leading 0 should not be ok
                self.current_length =
                    std::str::from_utf8(&length).unwrap().parse().unwrap();

                if self.current_length > self.max_length {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Packet length ({}) is larger than max_length {}.",
                            self.current_length, self.max_length
                        ),
                    ));
                }

                self.state = ParserState::Data;
            } else {
                // If len is 9 and we are still trying to parse the length,
                // give up now.
                // I absolutely don't want 99999999 sized packets.
                if buf.len() >= 9 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Data length part is bigger than 8.",
                    ));
                }
                return Ok(None);
            }
        }

        // In case we have already read the size of the data.
        if self.state == ParserState::Data {
            return self.parse_data(buf);
        }

        Ok(None)
    }

    fn parse_data(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<Vec<u8>>, io::Error> {
        if buf.len() >= self.current_length + 1 {
            let data = buf.split_to(self.current_length + 1);

            if data[data.len() - 1] != b',' {
                // There's a bug in the matrix.
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "End delimiter of data should be a comma",
                ));
            }

            // last char should be a comma.
            let data = &data[..data.len() - 1];

            self.state = ParserState::Length;
            self.current_length = 0;

            return Ok(Some(data.to_vec()));
        }

        Ok(None)
    }
}
