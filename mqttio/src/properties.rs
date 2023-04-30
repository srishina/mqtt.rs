use core::fmt;

use crate::{errors::Error, io::Reader, io::VarUint32Size, io::Writer};

enum_from_primitive! {
    #[derive(Debug, Copy, Clone)]
    pub enum PropertyID {
        PayloadFormatIndicator = 0x01,
        MessageExpiryInterval = 0x02,
        ContentType = 0x03,
        ResponseTopic = 0x08,
        CorrelationData = 0x09,
        SubscriptionIdentifier = 0x0B,
        SessionExpiryInterval = 0x11,
        AssignedClientIdentifier = 0x12,
        ServerKeepAlive = 0x13,
        AuthenticationMethod = 0x15,
        AuthenticationData = 0x16,
        RequestProblemInfo = 0x17,
        WillDelayInterval = 0x18,
        RequestResponseInfo = 0x19,
        ResponseInformation = 0x1A,
        ServerReference = 0x1C,
        ReasonString = 0x1F,
        ReceiveMaximum = 0x21,
        TopicAliasMaximum = 0x22,
        TopicAlias = 0x23,
        MaximumQoS = 0x24,
        RetainAvailable = 0x25,
        UserProperty = 0x26,
        MaximumPacketSize = 0x27,
        WildcardSubscriptionAvailable = 0x28,
        SubscriptionIdentifierAvailable = 0x29,
        SharedSubscriptionAvailable = 0x2A,
    }
}

impl PropertyID {
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyID::PayloadFormatIndicator => "Payload format indicator",
            PropertyID::MessageExpiryInterval => "Message expiry interval",
            PropertyID::ContentType => "Content type",
            PropertyID::ResponseTopic => "response topic",
            PropertyID::CorrelationData => "Correlation data",
            PropertyID::SubscriptionIdentifier => "Subscription Identifier",
            PropertyID::SessionExpiryInterval => "Session Expiry Interval",
            PropertyID::AssignedClientIdentifier => "Assigned Client Identifier",
            PropertyID::ServerKeepAlive => "Server Keep Alive",
            PropertyID::AuthenticationMethod => "Authentication Method",
            PropertyID::AuthenticationData => "Authentication Data",
            PropertyID::RequestProblemInfo => "Request Problem Information",
            PropertyID::RequestResponseInfo => "Request Response Information",
            PropertyID::WillDelayInterval => "Will Delay Interval",
            PropertyID::ResponseInformation => "Response Information",
            PropertyID::ServerReference => "Server Reference",
            PropertyID::ReasonString => "Reason String",
            PropertyID::ReceiveMaximum => "Receive Maximum",
            PropertyID::TopicAliasMaximum => "Topic Alias Maximum",
            PropertyID::TopicAlias => "Topic Alias",
            PropertyID::MaximumQoS => "Maximum QoS",
            PropertyID::RetainAvailable => "Retain Available",
            PropertyID::UserProperty => "User Property",
            PropertyID::MaximumPacketSize => "Maximum Packet Size",
            PropertyID::WildcardSubscriptionAvailable => "Wildcard Subscription Available",
            PropertyID::SubscriptionIdentifierAvailable => "Subscription Identifier Available",
            PropertyID::SharedSubscriptionAvailable => "Shared Subscription Available",
        }
    }
}

impl fmt::Display for PropertyID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PropertyID::{:?}", self)
    }
}

macro_rules! property_size {
    ($name:tt, $t:ty, varuint32) => {
        pub fn $name(value: &Option<$t>) -> u32
        where
            $t: Sized,
        {
            if value.is_some() {
                return VarUint32Size::size(value.unwrap()) + 1; // 1 for Property ID
            }
            return 0;
        }
    };
    ($name:tt, $t:ty, $size:expr) => {
        pub fn $name(value: &Option<$t>) -> u32
        where
            $t: Sized,
        {
            if value.is_some() {
                return $size + 1; // 1 for Property ID
            }
            return 0;
        }
    };
}

macro_rules! property_size_by_len {
    ($name:tt, $t:ty) => {
        pub fn $name(value: &$t) -> u32 {
            if !value.is_empty() {
                return value.len() as u32 + 3; // 1 for Property ID + 2 length (u16)
            }
            return 0;
        }
    };
}

pub struct PropertySize {}

impl PropertySize {
    property_size!(from_bool, bool, 1);
    property_size!(from_u8, u8, 1);
    property_size!(from_u16, u16, 2);
    property_size!(from_u32, u32, 4);

    property_size_by_len!(from_utf8_string, str);
    property_size_by_len!(from_binary_data, Vec<u8>);

    property_size!(from_varuint32, u32, varuint32);

    pub fn from_varuint32_array(arr: &[u32]) -> usize {
        if arr.len() == 0 {
            return 0;
        }
        let mut property_len: usize = 0;
        for d in arr {
            let n = VarUint32Size::size(*d);
            property_len += usize::try_from(n).unwrap() + 1; // 1 for Property ID
        }
        return property_len;
    }

    pub fn from_utf8_string_pair(arr: &[(String, String)]) -> u32 {
        if arr.len() == 0 {
            return 0;
        }
        let mut property_len: usize = 0;
        for d in arr {
            property_len += 4 + d.0.len() + d.1.len();
        }
        return property_len as u32;
    }
}

pub struct PropertyReader {}

macro_rules! property_reader_fn {
    ($name:tt, $t:ty, $read_method:ident) => {
        pub fn $name<R: Reader>(r: &mut R) -> Result<Option<$t>, Error> {
            let data = r.$read_method()?;
            Ok(Some(data))
        }
    };
    ($name:tt, $t:ty, $read_method:ident, no_option) => {
        pub fn $name<R: Reader>(r: &mut R) -> Result<$t, Error> {
            let data = r.$read_method()?;
            Ok(data)
        }
    };
}

impl PropertyReader {
    property_reader_fn!(to_bool, bool, read_bool);
    property_reader_fn!(to_u8, u8, read_u8);
    property_reader_fn!(to_u16, u16, read_u16);
    property_reader_fn!(to_u32, u32, read_u32);
    property_reader_fn!(to_varuint32, u32, read_varuint32);
    property_reader_fn!(to_utf8_string, String, read_utf8_string, no_option);
    property_reader_fn!(to_binary_data, Vec<u8>, read_binary, no_option);
    pub fn read_utf8_pair<R: Reader>(r: &mut R) -> Result<(String, String), Error> {
        let key = r.read_utf8_string()?;
        let value = r.read_utf8_string()?;
        return Ok((key, value));
    }
}

// PropertyWriter write the property when the value is not empty
pub struct PropertyWriter {}

macro_rules! property_writer_fn {
    ($name:tt, $t:ty, $write_method:ident) => {
        pub fn $name<W: Writer>(w: &mut W, id: PropertyID, v: &Option<$t>) -> Result<(), Error> {
            if !v.is_some() {
                return Ok(());
            }
            w.write_varuint32(id as u32)?;
            w.$write_method(v.unwrap())
        }
    };
    ($name:tt, $t:ty, $write_method:ident, no_option) => {
        pub fn $name<W: Writer>(w: &mut W, id: PropertyID, v: &$t) -> Result<(), Error> {
            if v.is_empty() {
                return Ok(());
            }
            w.write_varuint32(id as u32)?;
            w.$write_method(v)
        }
    };
}

impl PropertyWriter {
    property_writer_fn!(from_bool, bool, write_bool);
    property_writer_fn!(from_u8, u8, write_u8);
    property_writer_fn!(from_u16, u16, write_u16);
    property_writer_fn!(from_u32, u32, write_u32);
    property_writer_fn!(from_varuint32, u32, write_varuint32);
    property_writer_fn!(from_utf8_string, String, write_utf8_string, no_option);
    property_writer_fn!(from_binary_data, Vec<u8>, write_binary, no_option);
    pub fn from_utf8_pair<W: Writer>(
        w: &mut W,
        id: PropertyID,
        values: &[(String, String)],
    ) -> Result<(), Error> {
        if values.is_empty() {
            return Ok(());
        }
        for v in values {
            w.write_varuint32(id as u32)?;
            w.write_utf8_string(&v.0)?;
            w.write_utf8_string(&v.1)?;
        }
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::properties::{PropertyReader, PropertySize, PropertyWriter};

    use super::PropertyID;

    macro_rules! property_reader_helper {
        ($name:tt, $t:ty, $read_method:ident) => {
            fn $name(v: $t, data: &[u8]) {
                let mut cur = Cursor::new(data);
                let result = PropertyReader::$read_method(&mut cur);
                assert!(result.is_ok());
                let data = result.unwrap();
                assert!(data.is_some());
                assert_eq!(data.unwrap(), v);
            }
        };
        ($name:tt, $t:ty, $read_method:ident, no_opt) => {
            fn $name(v: $t, data: &[u8]) {
                let mut cur = Cursor::new(data);
                let result = PropertyReader::$read_method(&mut cur);
                assert!(result.is_ok());
                let data = result.unwrap();
                assert!(!data.is_empty());
                assert_eq!(data, v);
            }
        };
    }
    struct PropertyReaderHelper {}
    impl PropertyReaderHelper {
        property_reader_helper!(test_bool, bool, to_bool);
        property_reader_helper!(test_u8, u8, to_u8);
        property_reader_helper!(test_u16, u16, to_u16);
        property_reader_helper!(test_u32, u32, to_u32);
        property_reader_helper!(test_varuint32, u32, to_varuint32);
        property_reader_helper!(test_utf8_string, String, to_utf8_string, no_opt);
        property_reader_helper!(test_binary_data, Vec<u8>, to_binary_data, no_opt);
    }

    macro_rules! property_writer_helper {
        ($name:tt, $t:ty, $write_method:ident) => {
            fn $name(v: &Option<$t>, id: PropertyID, data: &[u8]) {
                let mut cur = Cursor::new(Vec::new());
                let result = PropertyWriter::$write_method(&mut cur, id, v);
                assert!(result.is_ok());
                assert_eq!(cur.get_ref(), data);
            }
        };
        ($name:tt, $t:ty, $write_method:ident, no_opt) => {
            fn $name(v: $t, id: PropertyID, data: &[u8]) {
                let mut cur = Cursor::new(Vec::new());
                let result = PropertyWriter::$write_method(&mut cur, id, &v);
                assert!(result.is_ok());
                assert_eq!(cur.get_ref(), data);
            }
        };
    }

    struct PropertyWriterHelper {}

    impl PropertyWriterHelper {
        property_writer_helper!(test_bool, bool, from_bool);
        property_writer_helper!(test_u8, u8, from_u8);
        property_writer_helper!(test_u16, u16, from_u16);
        property_writer_helper!(test_u32, u32, from_u32);
        property_writer_helper!(test_varuint32, u32, from_varuint32);
        property_writer_helper!(test_utf8_string, String, from_utf8_string, no_opt);
        property_writer_helper!(test_binary_data, Vec<u8>, from_binary_data, no_opt);
    }

    #[test]
    fn test_property_size() {
        assert_eq!(PropertySize::from_bool(&Some(true)), 2);
        assert_eq!(PropertySize::from_u8(&Some(8)), 2);
        assert_eq!(PropertySize::from_u16(&Some(128)), 3);
        assert_eq!(PropertySize::from_u32(&Some(1024)), 5);
        assert_eq!(PropertySize::from_utf8_string(&"hello"), 8);
        assert_eq!(PropertySize::from_binary_data(&vec![0x01, 0x02]), 5);
        assert_eq!(PropertySize::from_varuint32(&Some(127)), 2);
        assert_eq!(PropertySize::from_varuint32_array(&[127, 2097151]), 6);
        assert_eq!(
            PropertySize::from_utf8_string_pair(&[
                ("hello".to_string(), "world".to_string()),
                ("hello".to_string(), "world".to_string())
            ]),
            28
        );
    }

    #[test]
    fn test_property_reader() {
        PropertyReaderHelper::test_bool(true, [0x01].as_ref());
        PropertyReaderHelper::test_u8(0x80, [0x80].as_ref());
        PropertyReaderHelper::test_u16(128, u16::to_be_bytes(128).as_ref());
        PropertyReaderHelper::test_u32(8192, u32::to_be_bytes(8192).as_ref());
        PropertyReaderHelper::test_varuint32(16384, [0x80, 0x80, 0x01].as_ref());
        PropertyReaderHelper::test_utf8_string(
            "hello".to_string(),
            [0x00, 0x05, b'h', b'e', b'l', b'l', b'o'].as_ref(),
        );
        PropertyReaderHelper::test_binary_data(vec![0x01, 0x02], [0x00, 0x02, 0x01, 0x02].as_ref());
        {
            let mut cur = Cursor::new([
                0x00, 0x05, b'h', b'e', b'l', b'l', b'o', 0x00, 0x05, b'w', b'o', b'r', b'l', b'd',
            ]);
            let result = PropertyReader::read_utf8_pair(&mut cur);
            assert!(result.is_ok());
            let data = result.unwrap();
            assert_eq!(data.0, "hello");
            assert_eq!(data.1, "world");
        }
    }

    fn concat_u8(first: &[u8], second: &[u8]) -> Vec<u8> {
        [first, second].concat()
    }

    #[test]
    fn test_property_writer() {
        PropertyWriterHelper::test_bool(
            &Some(true),
            PropertyID::ContentType,
            [0x03, 0x01].as_ref(),
        );
        PropertyWriterHelper::test_u8(&Some(0x80), PropertyID::ContentType, [0x03, 0x80].as_ref());
        PropertyWriterHelper::test_u16(
            &Some(128),
            PropertyID::ContentType,
            concat_u8([0x03].as_ref(), u16::to_be_bytes(128).as_ref()).as_slice(),
        );
        PropertyWriterHelper::test_u32(
            &Some(8192),
            PropertyID::ContentType,
            concat_u8([0x03].as_ref(), u32::to_be_bytes(8192).as_ref()).as_slice(),
        );
        PropertyWriterHelper::test_varuint32(
            &Some(16384),
            PropertyID::ContentType,
            [0x03, 0x80, 0x80, 0x01].as_ref(),
        );
        PropertyWriterHelper::test_utf8_string(
            "hello".to_string(),
            PropertyID::ContentType,
            [0x03, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o'].as_ref(),
        );
        PropertyWriterHelper::test_binary_data(
            vec![0x01, 0x02],
            PropertyID::ContentType,
            [0x03, 0x00, 0x02, 0x01, 0x02].as_ref(),
        );
        {
            let mut cur = Cursor::new(Vec::new());
            let data = [
                0x03, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o', 0x00, 0x05, b'w', b'o', b'r', b'l',
                b'd',
            ];
            let result = PropertyWriter::from_utf8_pair(
                &mut cur,
                PropertyID::ContentType,
                &[("hello".to_string(), "world".to_string())],
            );
            assert!(result.is_ok());
            assert_eq!(cur.get_ref(), &data);
        }
    }
}
