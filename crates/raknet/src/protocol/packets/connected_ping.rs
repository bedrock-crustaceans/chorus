use crate::protocol::codec::RakCodec;
use crate::util::packet_id::CONNECTED_PING;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedPing {
    pub timestamp: u64,
}

impl RakCodec for ConnectedPing {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u8(CONNECTED_PING)?;
        writer.write_u64::<BigEndian>(self.timestamp)?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let id = reader.read_u8()?;
        if id != CONNECTED_PING {
            return Err(Error::new(ErrorKind::InvalidData, "not a ConnectedPing"));
        }

        let timestamp = reader.read_u64::<BigEndian>()?;

        Ok(Self { timestamp })
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>() + size_of::<u64>()
    }
}
