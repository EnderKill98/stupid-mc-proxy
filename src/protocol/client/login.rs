use std::io::{Cursor, Read};
use crate::protocol::{types::*, Packet};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientLoginStart {
    pub username: String,
    pub uuid: UUID,
}

impl Packet<Self> for ClientLoginStart {
    fn packet_id() -> VarInt {
        VarInt(0x00)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            username: String::read_as_mc_type(reader)?,
            uuid: UUID::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.username.write_as_mc_type(writer)?;
        self.uuid.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientLoginEncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl Packet<Self> for ClientLoginEncryptionResponse {
    fn packet_id() -> VarInt {
        VarInt(0x01)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            shared_secret: Vec::read_as_mc_type(reader)?,
            verify_token: Vec::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.shared_secret.write_as_mc_type(writer)?;
        self.verify_token.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientLoginPluginResponse {
    pub message_id: VarInt,
    pub successful: Boolean,
    pub data: Option<Vec<u8>>,
}

impl Packet<Self> for ClientLoginPluginResponse {
    fn packet_id() -> VarInt {
        VarInt(0x02)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        let message_id = VarInt::read_as_mc_type(reader)?;
        let successful = Boolean::read_as_mc_type(reader)?;
        let data = if successful {
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            Some(data)
        } else {
            None
        };

        Ok(Self {
            message_id,
            successful,
            data,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.message_id.write_as_mc_type(writer)?;
        self.successful.write_as_mc_type(writer)?;
        if self.successful {
            if let Some(data) = &self.data {
                writer.write_all(data)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// Response to LoginSuccess. After this the connection switches to configuration!
pub struct ClientLoginAcknowledged {}

impl Packet<Self> for ClientLoginAcknowledged {
    fn packet_id() -> VarInt {
        VarInt(0x03)
    }
    fn from_cursor(_reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {})
    }
    fn write_to(&self, _writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        Ok(())
    }
}
