use crate::protocol::codec::RakCodec;
use crate::util::constants::MAGIC;
use crate::util::packet_id::OPEN_CONNECTION_REPLY_2;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};
use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenConnectionReply2 {
    guid: u64,
    address: SocketAddr,
    mtu: u16,
    security: bool,
}

impl OpenConnectionReply2 {
    pub fn new(guid: u64, address: SocketAddr, mtu: u16, security: bool) -> Self {
        Self { guid, address, mtu, security }
    }
}

impl RakCodec for OpenConnectionReply2 {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u8(OPEN_CONNECTION_REPLY_2)?;
        writer.write_all(&MAGIC)?;
        writer.write_u64::<BigEndian>(self.guid)?;
        SocketAddr::serialize(&self.address, writer)?;
        writer.write_u16::<BigEndian>(self.mtu)?;
        writer.write_u8(self.security as u8)?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let id = reader.read_u8()?;
        if id != OPEN_CONNECTION_REPLY_2 {
            return Err(Error::new(ErrorKind::InvalidData, "not an OpenConnectionReply2"));
        }

        let mut magic = [0u8; MAGIC.len()];
        reader.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(Error::new(ErrorKind::InvalidData, "invalid magic"));
        }

        let guid = reader.read_u64::<BigEndian>()?;
        let address = SocketAddr::deserialize(reader)?;
        let mtu = reader.read_u16::<BigEndian>()?;
        let security = reader.read_u8()? != 0;

        Ok(Self { guid, address, mtu, security })
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>() + MAGIC.len() + size_of::<u64>() + SocketAddr::size_hint(&self.address) + size_of::<u16>() + size_of::<u8>()
    }
}
