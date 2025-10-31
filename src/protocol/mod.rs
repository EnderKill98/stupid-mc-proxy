use crate::protocol::types::MinecraftDataType;

use self::types::VarInt;
use anyhow::{bail, Context};
use std::convert::TryFrom;
use std::io::{Cursor, Read, Write};

pub mod client;
pub mod server;

#[allow(dead_code)]
pub mod types;

pub fn read_raw_packet_id_and_data(reader: &mut impl Read) -> anyhow::Result<(VarInt, Vec<u8>)> {
    // Get size (of packet id + packet data)
    let size = *VarInt::read_as_mc_type(reader)?;
    if size > 1024 * 1024 * 8 {
        bail!("Receiving packet size would be too big ({size} bytes > 8 MiB)!")
    }

    // Retrieve all data
    let mut buffer = vec![0u8; size as usize];
    reader
        .read_exact(&mut buffer)
        .context("Read expected packet size")?;
    let mut cursor = Cursor::new(buffer);

    // Read and check packet id
    let packet_id = VarInt::read_as_mc_type(&mut cursor).context("Read packet id")?;
    let (buffer_pos, mut buffer) = (cursor.position() as usize, cursor.into_inner());
    buffer.drain(..buffer_pos);
    Ok((packet_id, buffer))
}

pub trait Packet<T> {
    fn packet_id() -> types::VarInt;
    fn from_cursor(reader: &mut std::io::Cursor<&[u8]>) -> anyhow::Result<T>;
    fn read_with_header_from(reader: &mut impl Read) -> anyhow::Result<T> {
        let (packet_id, packet_data) = read_raw_packet_id_and_data(reader)?;
        if packet_id != Self::packet_id() {
            bail!(
                "Expected packet id {}, but got {}",
                Self::packet_id(),
                packet_id
            );
        }
        Self::from_cursor(&mut Cursor::new(packet_data.as_slice())).context("Parse packet")
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
