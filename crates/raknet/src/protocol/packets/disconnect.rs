use crate::protocol::codec::RakCodec;
use crate::util::packet_id::DISCONNECT;
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disconnect;

impl RakCodec for Disconnect {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u8(DISCONNECT)?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let id = reader.read_u8()?;
        if id != DISCONNECT {
            return Err(Error::new(ErrorKind::InvalidData, "not a Disconnect"));
        };

        Ok(Self)
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>()
    }
}
