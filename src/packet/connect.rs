use std::io::Cursor;

use crate::errors::Error;
use crate::propertyio_derive::IOOperations;

use mqttio::io::{BinaryData, KeyValuePair, Reader, UTF8String, VarUint32Size, Writer};
use mqttio::properties::{PropertyID, PropertyReader, PropertySize, PropertyWriter};
use num::FromPrimitive;

use super::packet::PacketType;

#[derive(Debug, Default, IOOperations)]
pub struct WillProperties {
    #[ioops(prop_id(PropertyID::WillDelayInterval))]
    will_delay_interval: Option<u32>,
    #[ioops(prop_id(PropertyID::PayloadFormatIndicator))]
    payload_format_indicator: Option<bool>,
    #[ioops(prop_id(PropertyID::MessageExpiryInterval))]
    message_expiry_interval: Option<u32>,
    #[ioops(prop_id(PropertyID::ContentType))]
    content_type: String,
    #[ioops(prop_id(PropertyID::ResponseTopic))]
    response_topic: String,
    #[ioops(prop_id(PropertyID::CorrelationData))]
    correlation_data: Vec<u8>,
    #[ioops(prop_id(PropertyID::UserProperty))]
    user_property: Vec<KeyValuePair>,
}

#[derive(Debug, Default)]
pub struct Will {
    qos: u8,
    retain: bool,
    properties: Option<WillProperties>,
    topic: String,
    payload: Vec<u8>,
}

impl Will {
    pub fn read<R: Reader>(r: &mut R, flag: u8) -> Result<Will, Error> {
        let mut will: Will = Default::default();
        will.qos = 0x03 & (flag >> 0x03);
        will.retain = (flag & 0x20) > 0;

        // Will properties
        will.properties = WillProperties::read(r)?;

        will.topic = r.read_utf8_string()?;
        will.payload = r.read_binary()?;

        return Ok(will);
    }
}

fn validate_connect_flag(flag: u8) -> Result<(), Error> {
    if flag & 0x01 != 0 {
        return Err(Error::InvalidConnectFlags);
    }

    let will_flag = (flag & 0x04) > 0;
    let will_qos = 0x03 & (flag >> 0x03);
    let will_retain = (flag & 0x20) > 0;

    // 3.1.2.6
    if (will_flag && (will_qos > 2)) || (!will_flag && will_qos != 0) {
        return Err(Error::InvalidWillQos);
    }

    // 3.1.2.7
    if !will_flag && will_retain {
        return Err(Error::InvalidWillRetain);
    }
    Ok(())
}

#[derive(Debug, Default, IOOperations)]
pub struct ConnectProperties {
    #[ioops(prop_id(PropertyID::SessionExpiryInterval))]
    session_expiry_interval: Option<u32>,
    #[ioops(prop_id(PropertyID::ReceiveMaximum))]
    receive_maximum: Option<u16>,
    #[ioops(prop_id(PropertyID::MaximumPacketSize))]
    maximum_packet_size: Option<u32>,
    #[ioops(prop_id(PropertyID::TopicAliasMaximum))]
    topic_alias_maximum: Option<u16>,
    #[ioops(prop_id(PropertyID::RequestProblemInfo))]
    request_problem_info: Option<bool>,
    #[ioops(prop_id(PropertyID::RequestResponseInfo))]
    request_response_info: Option<bool>,
    #[ioops(prop_id(PropertyID::UserProperty))]
    user_property: Vec<KeyValuePair>,
    #[ioops(prop_id(PropertyID::AuthenticationMethod))]
    authentication_method: String,
    #[ioops(prop_id(PropertyID::AuthenticationData))]
    authentication_data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Connect {
    protocol_name: &'static str,
    protocol_version: u8,
    clean_start: bool,
    keep_alive: u16,
    will: Option<Will>,
    properties: Option<ConnectProperties>,
    client_id: String,
    user_name: String,
    password: Vec<u8>,
}

impl Connect {
    pub fn read<R: Reader>(r: &mut R) -> Result<Connect, Error> {
        let pname = Reader::read_exact::<6>(r)?;
        if pname != [0, 4, b'M', b'Q', b'T', b'T'] {
            let v = match std::str::from_utf8(&pname) {
                Ok(v) => v,
                Err(_e) => "malformed content",
            };
            return Err(Error::InvalidProtocolName(v.to_string()));
        }
        let mut connect: Connect = Default::default();
        connect.protocol_name = "MQTT";

        connect.protocol_version = r.read_u8()?;
        if connect.protocol_version != 0x05 {
            return Err(Error::InvalidProtocolVersion);
        }

        let connect_flag = r.read_u8()?;

        validate_connect_flag(connect_flag)?;

        connect.clean_start = (connect_flag & 0x02) > 0;
        let will_flag = (connect_flag & 0x04) > 0;

        let password_flag = (connect_flag & 0x40) > 0;
        let username_flag = (connect_flag & 0x80) > 0;

        connect.keep_alive = r.read_u16()?;

        connect.properties = ConnectProperties::read(r)?;

        connect.client_id = r.read_utf8_string()?;

        if will_flag {
            println!("has will packet");
            let will = Will::read(r, connect_flag)?;
            connect.will = Some(will);
        }

        if username_flag {
            connect.user_name = r.read_utf8_string()?;
        }

        if password_flag {
            connect.password = r.read_binary()?;
        }

        return Ok(connect);
    }

    fn will_property_length(&self) -> u32 {
        if self.will.is_some() && self.will.as_ref().unwrap().properties.is_some() {
            return self
                .will
                .as_ref()
                .unwrap()
                .properties
                .as_ref()
                .unwrap()
                .len();
        }
        0
    }

    fn property_length(&self) -> u32 {
        if self.properties.is_some() {
            return self.properties.as_ref().unwrap().len();
        }
        0
    }

    pub fn write(&self) -> Result<Vec<u8>, Error> {
        let property_len = self.property_length();

        let will_property_len = self.will_property_length();

        // calculate the remaining length
        // 10 = protocolname + version + flags + keepalive
        let mut remaining_len = 10
            + property_len
            + VarUint32Size::size(property_len)
            + UTF8String::size(&self.client_id);

        let mut connect_flags: u8 = 0;
        if self.clean_start {
            connect_flags |= 0x02;
        }

        if self.will.is_some() {
            connect_flags |= 0x04; // Will flag
            let will = self.will.as_ref().unwrap();
            connect_flags |= will.qos << 0x03;
            if will.retain {
                connect_flags |= 0x20;
            }
            remaining_len += will_property_len + VarUint32Size::size(will_property_len);
            remaining_len += UTF8String::size(&will.topic) + BinaryData::size(&will.payload);
        }

        if self.user_name.len() > 0 {
            connect_flags |= 0x80;
            remaining_len += UTF8String::size(&self.user_name);
        }

        if self.password.len() > 0 {
            connect_flags |= 0x40;
            remaining_len += BinaryData::size(&self.password);
        }

        let remaining_len_usize = usize::try_from(remaining_len);
        if remaining_len_usize.is_err() {
            return Err(Error::InvalidRemaningLength(
                remaining_len_usize.unwrap_err(),
            ));
        }
        let mut packet = Cursor::new(Vec::<u8>::with_capacity(remaining_len_usize.unwrap()));
        packet.write_u8((PacketType::CONNECT as u8) << 0x04)?;
        packet.write_varuint32(remaining_len)?;

        packet.write_utf8_string("MQTT")?;
        packet.write_u8(0x05)?; // version

        packet.write_u8(connect_flags)?;

        packet.write_u16(self.keep_alive)?;

        packet.write_varuint32(property_len)?;

        if self.properties.is_some() {
            self.properties.as_ref().unwrap().write(&mut packet)?;
        }

        packet.write_utf8_string(&self.client_id)?;

        if self.will.is_some() {
            let will = self.will.as_ref().unwrap();
            packet.write_varuint32(will_property_len)?;
            if will.properties.is_some() {
                let will_props = will.properties.as_ref().unwrap();
                will_props.write(&mut packet)?;
            }
            packet.write_utf8_string(&will.topic)?;
            packet.write_binary(&will.payload)?;
        }

        if self.user_name.len() > 0 {
            packet.write_utf8_string(&self.user_name)?;
        }

        if self.password.len() > 0 {
            packet.write_binary(&self.password)?;
        }
        return Ok(packet.into_inner());
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use enum_primitive::FromPrimitive;

    use crate::{
        errors::Error,
        packet::packet::{FixedHeaderReader, PacketType},
    };

    use super::Connect;

    #[test]
    fn test_protocol_name_and_version() {
        let mut cur = Cursor::new([
            0x00, 0x04, b'M', b'Q', b'T', b'T', 0x05, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00,
        ]);
        let result = Connect::read(&mut cur);
        assert!(result.is_ok(), "{}", result.unwrap_err());
    }

    #[test]
    fn test_invalid_protocol_name() {
        let mut cur = Cursor::new([0x00, 0x04, b'M', b'Q', b'T', b'S', 0x05]);
        let result = Connect::read(&mut cur);
        assert!(std::matches!(
            result.unwrap_err(),
            Error::InvalidProtocolName { .. }
        ));
    }

    #[test]
    fn test_invalid_protocol_version() {
        let mut cur = Cursor::new([0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04]);
        let result = Connect::read(&mut cur);
        assert!(std::matches!(
            result.unwrap_err(),
            Error::InvalidProtocolVersion { .. }
        ));
    }

    #[test]
    fn test_invalid_connect_flags() {
        let data: [[u8; 8]; 3] = [
            [0x00, 0x04, b'M', b'Q', b'T', b'T', 0x05, 0x01],
            [0x00, 0x04, b'M', b'Q', b'T', b'T', 0x05, 0x1C], // 3.1.2.6
            [0x00, 0x04, b'M', b'Q', b'T', b'T', 0x05, 0x20], // 3.1.2.7
        ];
        for d in data.iter().enumerate() {
            let mut cur = Cursor::new(d.1);
            let result = Connect::read(&mut cur);
            if d.0 == 0 {
                assert!(matches!(
                    result.unwrap_err(),
                    Error::InvalidConnectFlags { .. }
                ));
            } else if d.0 == 1 {
                assert!(matches!(result.unwrap_err(), Error::InvalidWillQos { .. }));
            } else if d.0 == 2 {
                assert!(matches!(
                    result.unwrap_err(),
                    Error::InvalidWillRetain { .. }
                ));
            }
        }
    }

    #[test]
    fn test_connect_packet() {
        let data = [
            0x10, 0x1B, 0x00, 0x04, b'M', b'Q', b'T', b'T', 0x05, // protocol version
            0xC2, // Username=1, password=1, retain=0, qos=0, will=0, clean start=1, reserved=0
            0x00, 0x18, // Keep alive - 24
            0x00, // properties
            0x00, 0x00, // client id
            0x00, 0x05, b'h', b'e', b'l', b'l', b'o', // username
            0x00, 0x05, b'w', b'o', b'r', b'l', b'd', // username
        ];

        let mut cur = Cursor::new(data);

        let header_result = FixedHeaderReader::read(&mut cur);
        assert!(
            header_result.is_ok(),
            "Error reading fixed header {}",
            header_result.unwrap_err()
        );
        let hdr = header_result.unwrap();
        let packet_type = PacketType::from_u8(hdr.0 >> 4);
        assert!(packet_type.is_some());
        assert_eq!(PacketType::CONNECT, packet_type.unwrap());
        assert_eq!(hdr.1, 0x1B);

        let result = Connect::read(&mut cur);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let connect = result.unwrap();
        assert_eq!(connect.protocol_name, "MQTT");
        assert_eq!(connect.protocol_version, 0x05);
        assert!(connect.clean_start);
        assert_eq!(connect.keep_alive, 24);
        assert_eq!(connect.client_id, "");
        assert_eq!(connect.user_name, "hello");
        assert_eq!(connect.password, [b'w', b'o', b'r', b'l', b'd']);
        assert!(connect.properties.is_none());

        let written_result = connect.write();
        assert!(
            written_result.is_ok(),
            "Error writing CONNECT packet {}",
            written_result.unwrap_err()
        );
        assert_eq!(written_result.unwrap().as_slice(), data);
    }

    #[test]
    fn test_connect_packet_with_props() {
        let data = [
            0x10, 0x15, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, // MQTT
            0x05, // protocol version
            0x02, 0x00, 0x18, // Keep alive - 24
            0x08, // properties
            0x21, 0x00, 0x0A, // receive maximum
            0x27, 0x00, 0x00, 0x04, 0x00, // maximum packet size
            0x00, 0x00, // client id
        ];

        let mut cur = Cursor::new(data);
        let header_result = FixedHeaderReader::read(&mut cur);
        assert!(
            header_result.is_ok(),
            "Error reading fixed header {}",
            header_result.unwrap_err()
        );
        let hdr = header_result.unwrap();
        assert_eq!(hdr.1, 0x15);

        let result = Connect::read(&mut cur);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let connect = result.unwrap();
        assert!(connect.properties.is_some());
        let props = connect.properties.as_ref().unwrap();
        assert_eq!(props.receive_maximum, Some(10));
        assert_eq!(props.maximum_packet_size, Some(1024));

        let written_result = connect.write();
        assert!(
            written_result.is_ok(),
            "Error writing CONNECT packet with properties {}",
            written_result.unwrap_err()
        );
        assert_eq!(written_result.unwrap().as_slice(), data);
    }

    #[test]
    fn test_connect_packet_with_will_message() {
        let data = [
            0x10, 0x2A, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, // MQTT
            0x05, // protocol version
            0x0E, 0x00, 0x18, // Keep alive - 24
            0x08, // properties
            0x21, 0x00, 0x0A, // receive maximum
            0x27, 0x00, 0x00, 0x04, 0x00, // maximum packet size
            0x00, 0x00, // client id
            0x05, 0x18, 0x00, 0x00, 0x04, 0x00, 0x00, 0x03, 0x61, 0x2F, 0x62, 0x00, 0x08, 0x57,
            0x65, 0x6C, 0x63, 0x6F, 0x6D, 0x65, 0x21,
        ];
        let mut cur = Cursor::new(data);
        let header_result = FixedHeaderReader::read(&mut cur);
        assert!(
            header_result.is_ok(),
            "Error reading fixed header {}",
            header_result.unwrap_err()
        );
        let hdr = header_result.unwrap();
        assert_eq!(hdr.1, 0x2A);
        let result = Connect::read(&mut cur);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let connect = result.unwrap();
        assert!(connect.will.is_some());
        let will = connect.will.as_ref().unwrap();
        assert_eq!(will.qos, 0x01);
        assert!(!will.retain);
        assert_eq!(will.topic, "a/b");
        assert_eq!(will.payload, "Welcome!".as_bytes());
        assert!(will.properties.is_some());
        let will_props = will.properties.as_ref().unwrap();
        assert_eq!(will_props.will_delay_interval, Some(1024));

        let written_result = connect.write();
        assert!(
            written_result.is_ok(),
            "Error writing CONNECT packet with properties {}",
            written_result.unwrap_err()
        );
        assert_eq!(written_result.unwrap().as_slice(), data);
    }
}
