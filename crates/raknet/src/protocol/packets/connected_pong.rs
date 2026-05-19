use crate::protocol::codec::RakCodec;
use crate::util::packet_id::CONNECTED_PONG;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedPong {
    pub ping_timestamp: u64,
    pub timestamp: u64,
}

impl RakCodec for ConnectedPong {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u8(CONNECTED_PONG)?;
        writer.write_u64::<BigEndian>(self.ping_timestamp)?;
        writer.write_u64::<BigEndian>(self.timestamp)?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let id = reader.read_u8()?;
        if id != CONNECTED_PONG {
            return Err(Error::new(ErrorKind::InvalidData, "not a ConnectedPong"));
        }

        let ping_timestamp = reader.read_u64::<BigEndian>()?;
        let timestamp = reader.read_u64::<BigEndian>()?;

        Ok(Self { ping_timestamp, timestamp })
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>() + size_of::<u64>() + size_of::<u64>()
    }
}
