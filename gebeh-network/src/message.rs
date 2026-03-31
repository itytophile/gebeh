use rkyv::{Archive, Serialize};

#[derive(Archive, Serialize, Debug)]
pub(crate) struct MessageFromMaster {
    pub prediction: u8,
    pub first_message: (u8, u64),
    pub messages: Vec<(u8, u64)>,
}

#[derive(Archive, Serialize)]
pub(crate) struct MessageFromSlave {
    pub correction: u8,
    pub cycle: u64,
}

#[derive(Archive, Serialize)]
pub(crate) enum SerialMessage {
    FromMaster(MessageFromMaster),
    FromSlave(MessageFromSlave),
}

pub(crate) struct DecompressedSerialMessage {
    buffer: Vec<u8>,
}

impl DecompressedSerialMessage {
    pub fn get(&self) -> &ArchivedSerialMessage {
        rkyv::access::<ArchivedSerialMessage, rkyv::rancor::Error>(&self.buffer).unwrap()
    }
}

impl SerialMessage {
    pub fn deserialize(buffer: &[u8]) -> DecompressedSerialMessage {
        let decompressed = zstd::decode_all(buffer).unwrap();
        DecompressedSerialMessage {
            buffer: decompressed,
        }
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let serialized = rkyv::to_bytes::<rkyv::rancor::Error>(self).unwrap();
        let compressed = zstd::encode_all(&serialized[..], 0).unwrap();
        compressed.into_boxed_slice()
    }
}
