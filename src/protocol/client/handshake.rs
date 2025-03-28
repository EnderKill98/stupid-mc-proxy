use crate::protocol::{types::*, Packet};
use std::io::Cursor;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClientHandshake {
    pub protocol_version: VarInt,
    pub server_address: String,
    pub server_port: UnsignedShort,
    pub next_state: VarInt,
}

impl Packet<Self> for ClientHandshake {
    fn packet_id() -> VarInt {
        VarInt(0x00)
    }
    fn from_cursor(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Self> {
        Ok(Self {
            protocol_version: VarInt::read_as_mc_type(reader)?,
            server_address: String::read_as_mc_type(reader)?,
            server_port: UnsignedShort::read_as_mc_type(reader)?,
            next_state: VarInt::read_as_mc_type(reader)?,
        })
    }
    fn write_to(&self, writer: &mut impl std::io::Write) -> anyhow::Result<()> {
        self.protocol_version.write_as_mc_type(writer)?;
        self.server_address.write_as_mc_type(writer)?;
        self.server_port.write_as_mc_type(writer)?;
        self.next_state.write_as_mc_type(writer)?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_handshake_packet_against_itself() {
        let test_handshake = ClientHandshake {
            protocol_version: VarInt(799),
            server_address: "localhost".to_owned(),
            server_port: 25565,
            next_state: VarInt(1),
        };
        let mut data_cur = Cursor::new(Vec::<u8>::new());
        test_handshake.write_to(&mut data_cur).unwrap();
        data_cur.set_position(0);
        let read_back = ClientHandshake::from_cursor(&mut data_cur.into()).unwrap();
        assert_eq!(read_back, test_handshake);
    }
}
