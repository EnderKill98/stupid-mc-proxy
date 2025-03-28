use std::io::Cursor;
use crate::protocol::{types::*, Packet};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerStatusResponsePacket {
    pub json_response: String,
}

impl Packet<Self> for ServerStatusResponsePacket {
    fn packet_id() -> VarInt {
        VarInt(0x00)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            json_response: String::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.json_response.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerStatusPongPacket {
    pub payload: Long,
}

/// Note that sending this packet should also be followed by closing the connection!
impl Packet<Self> for ServerStatusPongPacket {
    fn packet_id() -> VarInt {
        VarInt(0x01)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            payload: Long::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.payload.write_as_mc_type(writer)?;
        Ok(())
    }
}
