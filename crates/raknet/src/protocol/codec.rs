use std::io::{Error, Read, Write};

pub trait RakCodec: Sized {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error>;

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error>;

    fn size_hint(&self) -> usize;
}
