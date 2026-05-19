use crate::protocol::codec::RakCodec;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

impl RakCodec for SocketAddr {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            SocketAddr::V4(addr) => {
                writer.write_u8(4)?;
                writer.write_all(&addr.ip().octets())?;
                writer.write_u16::<BigEndian>(addr.port())?;
            }
            SocketAddr::V6(addr) => {
                writer.write_u8(6)?;
                writer.write_u16::<BigEndian>(23)?; // AF_INET6
                writer.write_u16::<BigEndian>(addr.port())?;
                writer.write_u32::<BigEndian>(addr.flowinfo())?;
                writer.write_all(&addr.ip().octets())?;
                writer.write_u32::<BigEndian>(addr.scope_id())?;
            }
        }

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        match reader.read_u8()? {
            4 => {
                let mut octets = [0u8; 4];
                reader.read_exact(&mut octets)?;
                let ip = Ipv4Addr::from(octets);
                let port = reader.read_u16::<BigEndian>()?;

                Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
            }
            6 => {
                reader.read_u16::<BigEndian>()?; // AF_INET6
                let port = reader.read_u16::<BigEndian>()?;
                let flowinfo = reader.read_u32::<BigEndian>()?;
                let mut octets = [0u8; 16];
                reader.read_exact(&mut octets)?;
                let ip = Ipv6Addr::from(octets);
                let scope_id = reader.read_u32::<BigEndian>()?;

                Ok(SocketAddr::V6(SocketAddrV6::new(ip, port, flowinfo, scope_id)))
            }
            _ => Err(Error::new(ErrorKind::InvalidData, "invalid socket address")),
        }
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>()
            + match self {
                SocketAddr::V4(..) => 4 + size_of::<u16>(),
                SocketAddr::V6(..) => size_of::<u16>() + size_of::<u16>() + size_of::<u32>() + 16 + size_of::<u32>(),
            }
    }
}
