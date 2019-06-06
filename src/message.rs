use serde_derive::{Serialize, Deserialize};
use rmp_serde::Serializer;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub x: f32,
    pub msg: String,
}

impl Message {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }

    pub fn unpack(buf: Vec<u8>) -> Result<Self, rmp_serde::decode::Error> {
        Ok(rmp_serde::from_slice::<Message>(&buf).unwrap())
    }
}
