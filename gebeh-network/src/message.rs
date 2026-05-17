use rkyv::{Archive, Deserialize, Serialize, vec::ArchivedVec, with::AsVec};

use crate::synchro_cycles::NetworkCycle;

#[derive(Archive, Serialize, Deserialize, Debug)]
pub struct SerialMessage {
    pub is_master: bool,
    // the prediction field is more used like a session id in the slave case
    // if there a bad prediction somewhere then we can easily delete obsolete slave messages
    pub prediction: u8,
    pub value: u8,
    pub cycle: NetworkCycle,
}

pub struct DecompressedSerialMessage {
    buffer: Vec<u8>,
}

impl DecompressedSerialMessage {
    pub fn get(&self) -> &ArchivedVec<ArchivedSerialMessage> {
        rkyv::access::<ArchivedVec<ArchivedSerialMessage>, rkyv::rancor::Error>(&self.buffer)
            .unwrap()
    }
}

impl SerialMessage {
    pub fn deserialize(buffer: &[u8]) -> DecompressedSerialMessage {
        let decompressed = zstd::decode_all(buffer).unwrap();
        DecompressedSerialMessage {
            buffer: decompressed,
        }
    }

    pub fn serialize(messages: &[Self]) -> Box<[u8]> {
        #[derive(Archive, Serialize, Deserialize)]
        struct SliceWrapper<'a> {
            #[rkyv(with = AsVec)]
            slice: &'a [SerialMessage],
        }

        let wrapper = SliceWrapper { slice: messages };
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&wrapper).unwrap();
        zstd::encode_all(&bytes[..], 0).unwrap().into_boxed_slice()
    }
}
