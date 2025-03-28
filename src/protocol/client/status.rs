use std::io::Cursor;
use crate::protocol::{types::*, Packet};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientStatusRequest {}

impl Packet<Self> for ClientStatusRequest {
    fn packet_id() -> VarInt {
        VarInt(0x00)
    }
    fn from_cursor(_reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {})
    }
    fn write_to(&self, _writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientStatusPing {
    pub payload: Long,
}

impl Packet<Self> for ClientStatusPing {
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
