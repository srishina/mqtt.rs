use std::io::{self, Read};

use std::str::{self};

use crate::errors::Error;

pub type KeyValuePair = (String, String);

type BinaryType = Vec<u8>;

const MAX_VARUINT32: u32 = 268435455;
pub struct VarUint32Size {}

impl VarUint32Size {
    pub fn size(value: u32) -> u32 {
        let mut copied = value;
        let mut var_size: u32 = 0;
        loop {
            copied /= 0x80;
            var_size += 1;
            if copied == 0 {
                break;
            }
        }

        return var_size;
    }
}

pub struct UTF8String {}
impl UTF8String {
    pub fn size(value: &String) -> u32 {
        return value.len() as u32 + 2;
    }
}

pub struct BinaryData {}
impl BinaryData {
    pub fn size(value: &Vec<u8>) -> u32 {
        return value.len() as u32 + 2;
    }
}

fn validate_utf8_chars(v: &str) -> bool {
    for c in v.chars() {
        if c >= '\u{0000}' && c <= '\u{001f}' {
            return false;
        }

        if c >= '\u{007f}' && c <= '\u{009f}' {
            return false;
        }
    }
    true
}

pub trait Reader: io::Read {
    fn read_bool(&mut self) -> Result<bool, Error> {
        let v = self.read_u8()?;
        return Ok(v != 0);
    }

    fn read_u8(&mut self) -> Result<u8, Error> {
        let buf = Reader::read_exact::<1>(self)?;
        return Ok(buf[0]);
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        let buf = Reader::read_exact::<2>(self)?;
        return Ok(u16::from_be_bytes(buf));
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        let buf = Reader::read_exact::<4>(self)?;

        return Ok(u32::from_be_bytes(buf));
    }

    fn read_varuint32(&mut self) -> Result<u32, Error> {
        let mut value: u32 = 0;
        let mut multiplier: u32 = 1;
        let mut consumed: u32 = 0;

        loop {
            let encoded_byte = self.read_u8()?;
            consumed += 1;
            if consumed > 4 {
                return Err(Error::InvalidVarUint32(consumed));
            }

            value += (encoded_byte as u32 & 0x7f) * multiplier;
            if (encoded_byte & 0x80) == 0 {
                break;
            }

            multiplier *= 128;
            if multiplier > 128 * 128 * 128 {
                return Err(Error::InvalidVarUint32Length(multiplier));
            }
        }

        return Ok(value);
    }

    fn read_binary(&mut self) -> Result<BinaryType, Error> {
        let size = self.read_u16()?;

        let mut data: Vec<u8> = Vec::with_capacity(usize::from(size));
        data.resize(usize::from(size), 0);
        self.read_exact_buf(&mut data)?;
        return Ok(data);
    }

    fn read_utf8_string(&mut self) -> Result<String, Error> {
        let data = self.read_binary()?;
        match std::str::from_utf8(&data) {
            Ok(v) => {
                if validate_utf8_chars(v) {
                    return Ok(v.to_string());
                }
                return Err(Error::InvalidUTF8String);
            }
            Err(_e) => Err(Error::InvalidUTF8String),
        }
    }

    fn read_key_value_pair(&mut self) -> Result<KeyValuePair, Error> {
        let key = self.read_utf8_string()?;
        let value = self.read_utf8_string()?;
        return Ok((key, value));
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut buf: [u8; N] = [0; N];
        self.read_exact_buf(&mut buf)?;
        return Ok(buf);
    }

    fn read_exact_buf(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let result = Read::read_exact(self, buf);

        if result.is_err() {
            return Err(Error::MalformedPacket);
        }
        return Ok(());
    }
}

impl<R: io::Read + ?Sized> Reader for R {}

pub trait Writer: io::Write {
    fn write_bool(&mut self, value: bool) -> Result<(), Error> {
        self.write_internal(&[value as u8])
    }

    fn write_u8(&mut self, value: u8) -> Result<(), Error> {
        self.write_internal(&[value])
    }

    fn write_u16(&mut self, value: u16) -> Result<(), Error> {
        let buf: [u8; 2] = [(value >> 8) as u8, value as u8];
        self.write_internal(&buf)
    }

    fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        let buf: [u8; 4] = [
            (value >> 24) as u8,
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        ];
        self.write_internal(&buf)
    }

    fn write_varuint32(&mut self, value: u32) -> Result<(), Error> {
        if value > MAX_VARUINT32 {
            return Err(Error::InvalidVarUint32(value));
        }

        let mut copied = value;
        loop {
            let mut encoded_byte: u8 = (copied % 0x80) as u8;
            copied /= 0x80;
            if copied > 0 {
                encoded_byte |= 0x80;
            }
            self.write_u8(encoded_byte)?;
            if copied == 0 {
                break;
            }
        }
        return Ok(());
    }

    fn write_binary(&mut self, value: &[u8]) -> Result<(), Error> {
        self.write_u16(value.len() as u16)?;
        self.write_internal(value)
    }

    fn write_utf8_string(&mut self, value: &str) -> Result<(), Error> {
        self.write_u16(value.len() as u16)?;
        self.write_internal(value.as_bytes())
    }

    fn write_key_value_pair(&mut self, key: &str, value: &str) -> Result<(), Error> {
        self.write_utf8_string(key)?;
        self.write_utf8_string(value)
    }

    fn write_internal(&mut self, buf: &[u8]) -> Result<(), Error> {
        let result = self.write_all(buf);
        if result.is_err() {
            return Err(Error::MalformedPacket);
        }

        return Ok(());
    }
}

impl<W: io::Write + ?Sized> Writer for W {}

#[cfg(test)]
mod tests {

    use crate::errors::Error;
    use crate::io::MAX_VARUINT32;

    use super::Reader;
    use super::VarUint32Size;
    use super::Writer;
    use std::io::Cursor;

    trait HelperWriter {
        fn write<W: Writer>(&self, w: &mut W) -> Result<(), Error>;
    }

    trait HelperReader<T> {
        fn read<R: Reader>(r: &mut R) -> Result<T, Error>
        where
            T: Sized;
    }

    trait Tester {
        fn test(&self);
    }

    trait Getter<T> {
        fn get(&self) -> T;
        fn get_ref(&self) -> &T;
    }

    struct DefaultUint32(u32);
    struct VarUint32(u32);
    struct BinaryData(Vec<u8>);
    struct StringData(String);

    macro_rules! helper_getter {
        ($t:ty) => {
            impl Getter<$t> for $t {
                fn get(&self) -> $t {
                    return *self;
                }
                fn get_ref(&self) -> &$t {
                    return self;
                }
            }
        };
        ($t:ty, $rt:ty, struct0) => {
            impl Getter<$rt> for $t {
                fn get(&self) -> $rt {
                    return self.0;
                }
                fn get_ref(&self) -> &$rt {
                    return &self.0;
                }
            }
        };
        ($t:ty, $rt:ty, clonable) => {
            impl Getter<$rt> for $t {
                fn get(&self) -> $rt {
                    return self.0.clone();
                }
                fn get_ref(&self) -> &$rt {
                    return &self.0;
                }
            }
        };
    }

    helper_getter!(bool);
    helper_getter!(u8);
    helper_getter!(u16);
    helper_getter!(DefaultUint32, u32, struct0);
    helper_getter!(VarUint32, u32, struct0);
    helper_getter!(StringData, String, clonable);
    helper_getter!(BinaryData, Vec<u8>, clonable);

    #[derive(Default, Eq, PartialEq)]
    struct Adapter<T> {
        value: T,
    }

    macro_rules! helper_writer_by_ref {
        ($t:ty, $write_method:ident) => {
            impl HelperWriter for Adapter<$t> {
                fn write<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
                    w.$write_method(self.value.get_ref())
                }
            }
        };
    }

    macro_rules! helper_writer {
        ($t:ty, $write_method:ident) => {
            impl HelperWriter for Adapter<$t> {
                fn write<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
                    w.$write_method(self.value.get())
                }
            }
        };
    }

    macro_rules! helper_reader {
        ($t:ty, $rt:ty, $read_method:ident) => {
            impl HelperReader<$rt> for Adapter<$t> {
                fn read<R: Reader>(r: &mut R) -> Result<$rt, Error> {
                    r.$read_method()
                }
            }
        };
    }

    macro_rules! helper_tester {
        ($t:ty) => {
            impl Tester for Adapter<$t> {
                fn test(&self) {
                    let mut cur = Cursor::new(vec![0; 24]);
                    let written = self.write(&mut cur);
                    assert!(written.is_ok());
                    cur.set_position(0);
                    let result = Self::read(&mut cur);
                    assert!(result.is_ok());
                    assert_eq!(self.value.get_ref(), &result.unwrap());
                }
            }
        };
    }

    macro_rules! helper_rw {
        ($t:ty, $rt:ty, $write_method:ident, $read_method:ident) => {
            helper_writer!($t, $write_method);
            helper_reader!($t, $rt, $read_method);
            helper_tester!($t);
        };
    }

    macro_rules! helper_rw_by_ref {
        ($t:ty, $rt:ty, $write_method:ident, $read_method:ident) => {
            helper_writer_by_ref!($t, $write_method);
            helper_reader!($t, $rt, $read_method);
            helper_tester!($t);
        };
    }

    helper_rw!(bool, bool, write_bool, read_bool);
    helper_rw!(u8, u8, write_u8, read_u8);
    helper_rw!(u16, u16, write_u16, read_u16);
    helper_rw!(DefaultUint32, u32, write_u32, read_u32);
    helper_rw!(VarUint32, u32, write_varuint32, read_varuint32);

    helper_rw_by_ref!(StringData, String, write_utf8_string, read_utf8_string);
    helper_rw_by_ref!(BinaryData, Vec<u8>, write_binary, read_binary);

    #[test]
    fn test_byte_type() {
        let test_u8: Adapter<u8> = Adapter { value: 0x64 };
        test_u8.test();
    }

    #[test]
    fn test_bool_type() {
        let test_bool: Adapter<bool> = Adapter { value: true };
        test_bool.test();
    }

    #[test]
    fn test_u16_type() {
        let test_u16: Adapter<u16> = Adapter { value: 1024 };
        test_u16.test();
    }

    #[test]
    fn test_u32_type() {
        let test_u32: Adapter<DefaultUint32> = Adapter {
            value: DefaultUint32(8192),
        };
        test_u32.test();
    }

    #[test]
    fn test_varu32_type() {
        let data: [u32; 8] = [0, 127, 128, 16383, 16384, 2097151, 2097152, 268435455];
        for d in data {
            let test_var32: Adapter<VarUint32> = Adapter {
                value: VarUint32(d),
            };
            test_var32.test();
        }

        // Error cases
        {
            let mut cur = Cursor::new(vec![0x80, 0x80, 0x80, 0x80, 0x01]);
            let result = Reader::read_varuint32(&mut cur);
            assert!(
                result.is_err(),
                "VarUint32 reader did not return an error for a value that is above permissible"
            );
        }

        {
            let mut cur = Cursor::new(vec![0; 24]);
            let result = Writer::write_varuint32(&mut cur, MAX_VARUINT32 + 1);
            assert!(
                result.is_err(),
                "VarUint32 writer did not return an error for a value {} that is above permissible",
                MAX_VARUINT32 + 1
            );
        }
        {
            let mut cur = Cursor::new(Vec::new());
            let result = Reader::read_varuint32(&mut cur);
            assert!(
                result.is_err(),
                "VarUint32 reader did not return an error(underflow - EOF) for zero bytes input buffer"
            );
        }
        {
            let mut cur = Cursor::new(vec![0x80, 0x80, 0x80]);
            let result = Reader::read_varuint32(&mut cur);
            assert!(
                result.is_err(),
                "VarUint32 reader did not return an error(underflow - EOF) for an invalid input buffer"
            );
        }
    }

    #[test]
    fn test_string_type() {
        let data: [&str; 2] = ["hello world", "\u{FEFF}"];
        for d in data {
            println!("Len {}", d.len());
            let test_str: Adapter<StringData> = Adapter {
                value: StringData(d.to_string()),
            };
            test_str.test();
        }
    }

    #[test]
    fn test_binary_data_type() {
        let data = [
            vec!['h' as u8, 'e' as u8, 'l' as u8, 'l' as u8, 'o' as u8],
            vec![0xEF, 0xBB, 0xBF], // MQTT-1.5.4-3
        ];
        for d in data {
            let test_binary: Adapter<BinaryData> = Adapter {
                value: BinaryData(d),
            };
            test_binary.test();
        }
    }

    #[test]
    fn test_encoded_varuin32_size() {
        let data = [
            (0, 1),
            (127, 1),
            (128, 2),
            (16383, 2),
            (16384, 3),
            (2097151, 3),
            (2097152, 4),
            (268435455, 4),
        ];

        for d in data {
            let size = VarUint32Size::size(d.0);
            assert_eq!(
                size, d.1,
                "The encoded varuint32 size differs for {}, actual: {} expected: {}",
                d.0, size, d.1
            );
        }
    }

    #[test]
    fn test_valid_utf8_char() {
        fn test_char(c: char) {
            let s = c.to_string();
            let mut cur = Cursor::new(s.as_bytes());
            let result = Reader::read_utf8_string(&mut cur);
            assert!(result.is_err());
        }

        for c in '\u{0000}'..='\u{001f}' {
            test_char(c);
        }
        for c in '\u{007f}'..='\u{009f}' {
            test_char(c);
        }
    }
}
