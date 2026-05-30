#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RakReliability {
    Unreliable,
    UnreliableSequenced,
    Reliable,
    ReliableOrdered,
    ReliableSequenced,
    UnreliableWithAckReceipt,
    ReliableWithAckReceipt,
    ReliableOrderedWithAckReceipt,
}

impl RakReliability {
    pub fn is_reliable(&self) -> bool {
        match self {
            RakReliability::Reliable | RakReliability::ReliableOrdered | RakReliability::ReliableWithAckReceipt | RakReliability::ReliableOrderedWithAckReceipt => true,
            _ => false,
        }
    }

    pub fn is_sequenced(&self) -> bool {
        match self {
            RakReliability::ReliableSequenced | RakReliability::UnreliableSequenced => true,
            _ => false,
        }
    }

    pub fn is_ordered(&self) -> bool {
        match self {
            RakReliability::ReliableOrdered | RakReliability::ReliableOrderedWithAckReceipt => true,
            _ => false,
        }
    }
}

impl TryFrom<u8> for RakReliability {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RakReliability::Unreliable),
            1 => Ok(RakReliability::UnreliableSequenced),
            2 => Ok(RakReliability::Reliable),
            3 => Ok(RakReliability::ReliableOrdered),
            4 => Ok(RakReliability::ReliableSequenced),
            5 => Ok(RakReliability::UnreliableWithAckReceipt),
            6 => Ok(RakReliability::ReliableWithAckReceipt),
            7 => Ok(RakReliability::ReliableOrderedWithAckReceipt),
            _ => Err(()),
        }
    }
}

impl From<RakReliability> for u8 {
    fn from(value: RakReliability) -> Self {
        match value {
            RakReliability::Unreliable => 0,
            RakReliability::UnreliableSequenced => 1,
            RakReliability::Reliable => 2,
            RakReliability::ReliableOrdered => 3,
            RakReliability::ReliableSequenced => 4,
            RakReliability::UnreliableWithAckReceipt => 5,
            RakReliability::ReliableWithAckReceipt => 6,
            RakReliability::ReliableOrderedWithAckReceipt => 7,
        }
    }
}
