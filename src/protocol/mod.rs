use crate::protocol::types::MinecraftDataType;

use self::types::VarInt;
use anyhow::{bail, Context};
use std::convert::TryFrom;
use std::io::{Cursor, Read};

pub mod client;
pub mod server;
pub mod types;

pub trait Packet<T> {
    fn packet_id() -> types::VarInt;
    fn from_cursor(reader: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<T>;
    fn read_with_header_from(reader: &mut impl Read) -> anyhow::Result<T> {
        // Get size (of packet id + packet data)
        let size = *VarInt::read_as_mc_type(reader)?;
        if size > 1024 * 1024 * 8 {
            bail!("Receiving packet size would be too big ({size} bytes > 8 MiB)!")
        }

        // Retreive all data
        let mut buffer = vec![0u8; size as usize];
        reader
            .read_exact(&mut buffer)
            .context("Read expected packet size")?;
        let mut cursor = Cursor::new(buffer);

        // Read and check packet id
        let packet_id = VarInt::read_as_mc_type(&mut cursor).context("Read packet id")?;
        if packet_id != Self::packet_id() {
            bail!(
                "Expected packet id {}, but got {}",
                Self::packet_id(),
                packet_id
            );
        }

        // Create packet
        let (buffer_pos, buffer) = (cursor.position() as usize, cursor.into_inner());
        let mut cursor = Cursor::new(&buffer[buffer_pos..]);
        Self::from_cursor(&mut cursor).context("Parse packet")
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()>;
    fn write_with_header_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        let mut content = std::io::Cursor::new(Vec::<u8>::new());
        Self::packet_id().write_as_mc_type(&mut content)?;
        self.write_to(&mut content)?;
        VarInt(i32::try_from(content.position())?).write_as_mc_type(writer)?;
        writer.write_all(&mut content.into_inner())?;
        Ok(())
    }
}
