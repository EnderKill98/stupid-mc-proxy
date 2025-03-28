use anyhow::{bail, ensure, Result};
use std::convert::TryFrom;
use std::fmt::Display;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
//use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
// Many types are quite pointless. They are mainly just for matching with wiki.vg / Java

pub type Boolean = bool;
pub type Byte = i8;
pub type UnsignedByte = u8;
pub type Short = i16;
pub type UnsignedShort = u16;
pub type Int = i32;
pub type Long = i64;
pub type Float = f32;
pub type Double = f64;
pub type String = std::string::String;
//pub type Chat = chat_formatting::chat::Chat;
//#[derive(Debug, Clone, PartialEq, Eq)]
//pub struct NbtChat(pub Chat);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct VarInt(pub i32);
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct VarLong(pub i64);
// Entity Metadata
// Slot

//pub type NbtTag = simdnbt::owned::NbtTag;
//pub type NbtTagCompound = simdnbt::owned::NbtCompound;

// Position
// Angle
pub type UUID = uuid::Uuid;
// Array of X => Vec<MinecraftDataType>
// X Enum
pub trait MinecraftDataType: Sized {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self>;
    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()>;
}

impl Deref for VarInt {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VarInt {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for VarInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VarInt({})", self.0)
    }
}

impl MinecraftDataType for VarInt {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut num_read: usize = 0;
        let mut result: i32 = 0;
        let mut read = [0xFFu8; 1];
        while read[0] & 0b10000000 != 0 {
            reader.read_exact(&mut read)?;
            let value = (read[0] & 0b01111111) as i32;
            result |= value << (7 * num_read);

            num_read += 1;
            if num_read > 5 {
                bail!("VarInt is too big");
            }
        }

        Ok(VarInt(result))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut value = self.0 as u32; // Treat as unsigned
        let mut bytes: Vec<u8> = Vec::with_capacity(5);
        loop {
            if (value & 0xFFFFFF80) == 0 {
                bytes.push((value & 0xFF) as u8);
                return Ok(writer.write_all(&mut bytes)?);
            }

            bytes.push(((value & 0x7F | 0x80) & 0xFF) as u8);
            // Note: >>> means that the sign bit is shifted with the rest of the number rather than being left alone
            value >>= 7;
        }
    }
}

impl VarInt {
    /*pub async fn async_read_as_mc_type<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Self> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut num_read: usize = 0;
        let mut result: i32 = 0;
        let mut read = [0xFFu8; 1];
        while read[0] & 0b10000000 != 0 {
            reader.read_exact(&mut read).await?;
            let value = (read[0] & 0b01111111) as i32;
            result |= value << (7 * num_read);

            num_read += 1;
            if num_read > 5 {
                bail!("VarInt is too big");
            }
        }

        Ok(VarInt(result))
    }

    pub async fn async_write_as_mc_type<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<()> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut value = self.0 as u32; // Treat as unsigned
        let mut bytes: Vec<u8> = Vec::with_capacity(5);
        loop {
            if (value & 0xFFFFFF80) == 0 {
                bytes.push((value & 0xFF) as u8);
                return Ok(writer.write_all(&mut bytes).await?);
            }

            bytes.push(((value & 0x7F | 0x80) & 0xFF) as u8);
            // Note: >>> means that the sign bit is shifted with the rest of the number rather than being left alone
            value >>= 7;
        }
    }*/
}

impl Display for VarLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VarLong({})", self.0)
    }
}

impl MinecraftDataType for VarLong {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut num_read: usize = 0;
        let mut result: i64 = 0;
        let mut read = [0xFFu8; 1];
        while read[0] & 0b10000000 != 0 {
            reader.read_exact(&mut read)?;
            let value = (read[0] & 0b01111111) as i64;
            result |= value << (7 * num_read);

            num_read += 1;
            if num_read > 10 {
                bail!("VarLong is too big");
            }
        }

        Ok(VarLong(result))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Taken from https://wiki.vg/Protocol#VarInt_and_VarLong
        let mut value = self.0 as u64; // Treat as unsigned
        let mut bytes: Vec<u8> = Vec::with_capacity(5);
        loop {
            if (value & 0xFFFFFFFFFFFFFF80) == 0 {
                bytes.push((value & 0xFF) as u8);
                return Ok(writer.write_all(&mut bytes)?);
            }

            bytes.push(((value & 0x7F | 0x80) & 0xFF) as u8);
            // Note: >>> means that the sign bit is shifted with the rest of the number rather than being left alone
            value >>= 7;
        }
    }
}

impl MinecraftDataType for Boolean {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 1];
        reader.read_exact(&mut data)?;
        Ok(data[0] != 0x00)
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[if *self { 0x01 } else { 0x00 }])?;
        Ok(())
    }
}

impl MinecraftDataType for Byte {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 1];
        reader.read_exact(&mut data)?;
        Ok(data[0] as Byte)
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[*self as u8])?;
        Ok(())
    }
}

impl MinecraftDataType for UnsignedByte {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 1];
        reader.read_exact(&mut data)?;
        Ok(data[0])
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[*self])?;
        Ok(())
    }
}

impl MinecraftDataType for Short {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 2];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for UnsignedShort {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 2];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for Int {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 4];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for Long {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 8];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for Float {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 4];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for Double {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 8];
        reader.read_exact(&mut data)?;
        Ok(Self::from_be_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_be_bytes())?;
        Ok(())
    }
}

impl MinecraftDataType for UUID {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = [0u8; 16];
        reader.read_exact(&mut data)?;
        Ok(Self::from_bytes(data))
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}

/// Var Arrays. Any kind of Mincraft DataType array.
/// Not really official and expected to be prefixed
/// by a VarInt declaring the length of the array.
impl<T: MinecraftDataType> MinecraftDataType for Vec<T> {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let array_length = *VarInt::read_as_mc_type(reader)?;
        ensure!(array_length >= 0, "Length can't be less than 0!");
        let mut array = Vec::with_capacity(array_length as usize);
        for _ in 0..array_length {
            array.push(T::read_as_mc_type(reader)?);
        }
        Ok(array)
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        VarInt(i32::try_from(self.len())?).write_as_mc_type(writer)?;
        for element in self.iter() {
            element.write_as_mc_type(writer)?;
        }
        Ok(())
    }
}

/// Optional types. A boolean is sent first to indicate if they exist or not
impl<T: MinecraftDataType> MinecraftDataType for Option<T> {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let is_present = Boolean::read_as_mc_type(reader)?;
        return Ok(if is_present {
            Some(T::read_as_mc_type(reader)?)
        } else {
            None
        });
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.is_some().write_as_mc_type(writer)?;
        if let Some(value) = self {
            value.write_as_mc_type(writer)?;
        }
        Ok(())
    }
}

impl MinecraftDataType for String {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let string_length = *VarInt::read_as_mc_type(reader)?;
        ensure!(string_length >= 0, "Length can't be less than 0!");
        let mut string_bytes = vec![0u8; string_length as usize];
        reader.read_exact(&mut string_bytes)?;
        Ok(String::from_utf8(string_bytes)?)
    }

    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        VarInt(i32::try_from(self.as_bytes().len())?).write_as_mc_type(writer)?;
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn test_var_int_against_itself() {
        for test_number in &[
            VarInt(0),
            VarInt(1),
            VarInt(100),
            VarInt(255),
            VarInt(256),
            VarInt(1000),
            VarInt(12345),
            VarInt(2000123),
            VarInt(i32::MAX),
            VarInt(-1),
            VarInt(-123),
            VarInt(i32::MIN),
        ] {
            let mut data_cur = Cursor::new(Vec::<u8>::new());
            test_number.write_as_mc_type(&mut data_cur).unwrap();
            data_cur.set_position(0);
            let read_back = VarInt::read_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                read_back, *test_number,
                "Ensuring the test number {} can be written and read to the same number",
                test_number
            );
        }
    }

    #[test]
    fn test_var_int_against_wiki_samples() {
        for (value, hex_bytes) in &[
            (VarInt(0), vec![0x00]),
            (VarInt(1), vec![0x01]),
            (VarInt(2), vec![0x02]),
            (VarInt(127), vec![0x7f]),
            (VarInt(128), vec![0x80, 0x01]),
            (VarInt(255), vec![0xff, 0x01]),
            (VarInt(2097151), vec![0xff, 0xff, 0x7f]),
            (VarInt(2147483647), vec![0xff, 0xff, 0xff, 0xff, 0x07]),
            (VarInt(-1), vec![0xff, 0xff, 0xff, 0xff, 0x0f]),
            (VarInt(-2147483648), vec![0x80, 0x80, 0x80, 0x80, 0x08]),
        ] {
            // Read
            let mut data_cur = Cursor::new(hex_bytes);
            let read_back = VarInt::read_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                read_back, *value,
                "Checking that read example bytes are actually the expected value ({})",
                *value
            );
            // Write
            let mut data_cur = Cursor::new(Vec::<u8>::new());
            value.write_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                data_cur.into_inner(),
                *hex_bytes,
                "Checking written data for value {} against wiki sample bytes",
                *value
            );
        }
    }

    #[test]
    fn test_var_long_against_itself() {
        for test_number in &[
            VarLong(0),
            VarLong(1),
            VarLong(100),
            VarLong(255),
            VarLong(256),
            VarLong(1000),
            VarLong(12345),
            VarLong(2000123),
            VarLong(20000000123),
            VarLong(-20000000123),
            VarLong(i64::MAX),
            VarLong(-1),
            VarLong(-123),
            VarLong(i64::MIN),
        ] {
            let mut data_cur = Cursor::new(Vec::<u8>::new());
            test_number.write_as_mc_type(&mut data_cur).unwrap();
            data_cur.set_position(0);
            let read_back = VarLong::read_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                read_back, *test_number,
                "Ensuring the test number {} can be written and read to the same number",
                test_number
            );
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_var_long_against_wiki_samples() {
        for (value, hex_bytes) in &[
            (VarLong(0), vec![0x00]),
            (VarLong(1), vec![0x01]),
            (VarLong(2), vec![0x02]),
            (VarLong(127), vec![0x7f]),
            (VarLong(128), vec![0x80, 0x01]),
            (VarLong(255), vec![0xff, 0x01]),
            (VarLong(2147483647), vec![0xff, 0xff, 0xff, 0xff, 0x07]),
            (VarLong(9223372036854775807), vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
            (VarLong(-1), vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]),
            (VarLong(-2147483648), vec![0x80, 0x80, 0x80, 0x80, 0xf8, 0xff, 0xff, 0xff, 0xff, 0x01]),
            (VarLong(-9223372036854775808), vec![0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]),
        ] {
            // Read
            let mut data_cur = Cursor::new(hex_bytes);
            let read_back = VarLong::read_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                read_back, *value,
                "Checking that read example bytes are actually the expected value ({})",
                *value
            );
            // Write
            let mut data_cur = Cursor::new(Vec::<u8>::new());
            value.write_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                data_cur.into_inner(),
                *hex_bytes,
                "Checking written data for value {} against wiki sample bytes",
                *value
            );
        }
    }

    #[test]
    fn test_string_against_itself() {
        for test_string in ["", "1", "Hello", "test@something", "A\nBC\tDEF"] {
            let mut data_cur = Cursor::new(Vec::<u8>::new());
            test_string
                .to_owned()
                .write_as_mc_type(&mut data_cur)
                .unwrap();
            data_cur.set_position(0);
            let read_back = String::read_as_mc_type(&mut data_cur).unwrap();
            assert_eq!(
                test_string, &read_back,
                "Ensuring the same written text is also read back as the same (tested {:?})",
                test_string
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub namespace: String,
    pub path: String,
}

impl FromStr for Identifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let split: Vec<_> = s.split(':').collect();
        ensure!(split.len() == 2, "Identifier must have exactly one colon");
        Ok(Identifier::new(split[0], split[1]))
    }
}

impl Identifier {
    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
        }
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl MinecraftDataType for Identifier {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(String::read_as_mc_type(reader)?.parse()?)
    }
    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.to_string().write_as_mc_type(writer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Angle(pub Byte);

impl MinecraftDataType for Angle {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Angle(Byte::read_as_mc_type(reader)?))
    }
    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.0.write_as_mc_type(writer)
    }
}

impl From<f32> for Angle {
    fn from(f: f32) -> Self {
        Angle((f / 360f32 * 256f32) as i8)
    }
}

impl From<Angle> for f32 {
    fn from(a: Angle) -> Self {
        a.0 as f32 / (256f32 / 360f32)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    x: i32,
    y: i16,
    z: i32,
}

/// NOT REALLY TESTED YET
impl MinecraftDataType for Position {
    fn read_as_mc_type<R: Read>(reader: &mut R) -> Result<Self> {
        let val = Long::read_as_mc_type(reader)? as u64;
        let x = (val >> 38) as i32;
        let y = (val << 52 >> 52) as i16;
        let z = (val << 26 >> 38) as i32;
        Ok(Position { x, y, z })
    }
    fn write_as_mc_type<W: Write>(&self, writer: &mut W) -> Result<()> {
        let encoded: u64 = ((self.x as u64 & 0x3FFFFFF) << 38)
            | ((self.z as u64 & 0x3FFFFFF) << 12)
            | (self.y as u64 & 0xFFF);
        (encoded as i64).write_as_mc_type(writer)?;
        Ok(())
    }
}
