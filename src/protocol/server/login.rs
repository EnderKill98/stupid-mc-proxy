use crate::protocol::{types::*, Packet};
use std::io::{Cursor, Read};

/// Disconnect packet before logged in
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerLoginDisconnect {
    pub reason: serde_json::Value,
}

impl Packet<Self> for ServerLoginDisconnect {
    fn packet_id() -> VarInt {
        VarInt(0x00)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            reason: serde_json::from_str(&String::read_as_mc_type(reader)?)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        serde_json::to_string(&self.reason)?.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerLoginEncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
    pub should_authenticate: Boolean,
}

impl Packet<Self> for ServerLoginEncryptionRequest {
    fn packet_id() -> VarInt {
        VarInt(0x01)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            server_id: String::read_as_mc_type(reader)?,
            public_key: Vec::read_as_mc_type(reader)?,
            verify_token: Vec::read_as_mc_type(reader)?,
            should_authenticate: Boolean::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.server_id.write_as_mc_type(writer)?;
        self.public_key.write_as_mc_type(writer)?;
        self.verify_token.write_as_mc_type(writer)?;
        self.should_authenticate.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerLoginSuccess {
    pub uuid: UUID,
    pub username: String,
}

impl Packet<Self> for ServerLoginSuccess {
    fn packet_id() -> VarInt {
        VarInt(0x02)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            uuid: UUID::read_as_mc_type(reader)?,
            username: String::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.uuid.write_as_mc_type(writer)?;
        self.username.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerLoginPluginRequest {
    pub message_id: VarInt,
    /// TODO: Change type to Identifier
    pub channel: String,
    pub data: Vec<u8>,
}

impl Packet<Self> for ServerLoginPluginRequest {
    fn packet_id() -> VarInt {
        VarInt(0x04)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        let message_id = VarInt::read_as_mc_type(reader)?;
        let channel = String::read_as_mc_type(reader)?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        Ok(Self {
            message_id,
            channel,
            data,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.message_id.write_as_mc_type(writer)?;
        self.channel.write_as_mc_type(writer)?;
        writer.write_all(&self.data)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerLoginSetCompression {
    /// Enable zlib compression if value is 1 or greater
    pub threshold: VarInt,
}

impl Packet<Self> for ServerLoginSetCompression {
    fn packet_id() -> VarInt {
        VarInt(0x03)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            threshold: VarInt::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.threshold.write_as_mc_type(writer)?;
        Ok(())
    }
}
