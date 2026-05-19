use crate::protocol::codec::RakCodec;
use crate::util::constants::MAGIC;
use crate::util::packet_id::OPEN_CONNECTION_REQUEST_2;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Error, ErrorKind, Read, Write};
use std::net::SocketAddr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenConnectionRequest2 {
    pub cookie: Option<i32>,
    pub addr: SocketAddr,
    pub mtu: u16,
    pub client: u64,
}

impl RakCodec for OpenConnectionRequest2 {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u8(OPEN_CONNECTION_REQUEST_2)?;
        writer.write_all(&MAGIC)?;
        match self.cookie {
            Some(cookie) => {
                writer.write_i32::<BigEndian>(cookie)?;
                writer.write_u8(0)?; // no security challenge
            }
            None => (),
        }
        SocketAddr::serialize(&self.addr, writer)?;
        writer.write_u16::<BigEndian>(self.mtu)?;
        writer.write_u64::<BigEndian>(self.client)?;

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        // Security:
        // - Enabled = 5
        // - Disabled = 0
        // Address:
        // - IPv4 = 7
        // - IPv6 = 29
        // Remaining = 10

        // IPv4 (& Remaining) = 17
        // IPv4 & Security (& Remaining) = 22
        // IPv6 (& Remaining) = 39
        // IPv6 & Security (& Remaining) = 44

        // If remaining size after deserializing magic is 22 or 44 bytes
        // then the client has sent a cookie (security) in the request.
        // Otherwise, if the size is 17 or 39, there is no cookie.

        let id = reader.read_u8()?;
        if id != OPEN_CONNECTION_REQUEST_2 {
            return Err(Error::new(ErrorKind::InvalidData, "not an OpenConnectionRequest2"));
        }

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;

        let mut reader = Cursor::new(buf);

        let mut magic = [0u8; MAGIC.len()];
        reader.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(Error::new(ErrorKind::InvalidData, "invalid magic"));
        }

        let cookie = match reader.get_ref().len() - reader.position() as usize {
            22 | 44 => {
                let value = reader.read_i32::<BigEndian>()?;
                reader.read_u8()?; // ignore security challenge
                Some(value)
            }
            _ => None,
        };
        let address = SocketAddr::deserialize(&mut reader)?;
        let mtu = reader.read_u16::<BigEndian>()?;
        let client = reader.read_u64::<BigEndian>()?;

        Ok(Self { cookie, addr: address, mtu, client })
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>()
            + MAGIC.len()
            + match self.cookie {
                Some(_) => size_of::<i32>() + size_of::<u8>(),
                None => 0,
            }
            + SocketAddr::size_hint(&self.addr)
            + size_of::<u16>()
            + size_of::<u64>()
    }
}
