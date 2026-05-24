use crate::protocol::codec::RakCodec;
use crate::protocol::types::frame::Frame;
use crate::util::flags::{CONTINUOUS_SEND, NEEDS_B_AND_AS, PAIR, VALID};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error, ErrorKind, Read, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameSet {
    pub sequence: u32,
    pub frames: Vec<Frame>,
    pub continuous_send: bool,
    pub needs_b_and_as: bool,
    pub is_pair: bool,
}

impl FrameSet {
    pub fn new(sequence: u32, frames: Vec<Frame>, continuous_send: bool, needs_b_and_as: bool, is_pair: bool) -> Self {
        Self {
            sequence,
            frames,
            continuous_send,
            needs_b_and_as,
            is_pair,
        }
    }
}

impl RakCodec for FrameSet {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        let mut flags = VALID;
        if self.continuous_send {
            flags |= CONTINUOUS_SEND;
        }
        if self.needs_b_and_as {
            flags |= NEEDS_B_AND_AS;
        }
        if self.is_pair {
            flags |= PAIR;
        }

        writer.write_u8(flags)?;
        writer.write_u24::<LittleEndian>(self.sequence)?;
        for frame in &self.frames {
            Frame::serialize(&frame, writer)?;
        }

        Ok(())
    }

    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let flags = reader.read_u8()?;
        if flags & VALID != VALID {
            return Err(Error::new(ErrorKind::InvalidData, "not a FrameSet"));
        }

        let sequence = reader.read_u24::<LittleEndian>()?;

        let mut frames = Vec::new();
        loop {
            match Frame::deserialize(reader) {
                Ok(frame) => frames.push(frame),
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        break;
                    }
                    return Err(e);
                }
            }
        }

        let continuous_send = flags & CONTINUOUS_SEND != 0;
        let needs_b_and_as = flags & NEEDS_B_AND_AS != 0;
        let is_pair = flags & PAIR != 0;

        Ok(Self::new(sequence, frames, continuous_send, needs_b_and_as, is_pair))
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>() + 3 + self.frames.iter().fold(0, |acc, frame| acc + Frame::size_hint(&frame))
    }
}
