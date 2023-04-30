use mqttio::io::Reader;

use crate::errors::Error;

// PacketType MQTT control packet type
// MQTT 12.1.2
enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum PacketType {
        RESERVED = 0,
        CONNECT = 1,
        CONNACK = 2,
        PUBLISH = 3,
        PUBACK = 4,
        PUBREC = 5,
        PUBREL = 6,
        PUBCOMP = 7,
        SUBSCRIBE = 8,
        SUBACK = 9,
        UNSUBSCRIBE = 10,
        UNSUBACK = 11,
        PINGREQ = 12,
        PINGRESP = 13,
        DISCONNECT = 14,
        AUTH = 15,
    }
}

impl PacketType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PacketType::RESERVED => "RESERVED for future use",
            PacketType::CONNECT => "CONNECT",
            PacketType::CONNACK => "CONNACK",
            PacketType::PUBLISH => "PUBLISH",
            PacketType::PUBACK => "PUBACK",
            PacketType::PUBREC => "PUBREC",
            PacketType::PUBREL => "PUBREL",
            PacketType::PUBCOMP => "PUBCOMP",
            PacketType::SUBSCRIBE => "SUBSCRIBE",
            PacketType::SUBACK => "SUBACK",
            PacketType::UNSUBSCRIBE => "UNSUBSCRIBE",
            PacketType::UNSUBACK => "UNSUBACK",
            PacketType::PINGREQ => "PINGREQ",
            PacketType::PINGRESP => "PINGRESP",
            PacketType::DISCONNECT => "DISCONNECT",
            PacketType::AUTH => "AUTH",
        }
    }
}

// ReasonCode MQTT reason code that indicates the result of an operation
// MQTT sec 2.4. Only the reasoncodes that are common across the MQTT packets
// are defined here. The specific packet based error codes can be found in their
// respective packet writer/reader

pub enum ReasonCode {
    Success = 0x00, // CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, UNSUBACK, AUTH
    NoMatchingSubscribers = 0x10, // PUBACK, PUBREC
    UnspecifiedError = 0x80, // CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    MalformedPacket = 0x81, // CONNACK, DISCONNECT
    ProtocolError = 0x82, // CONNACK, DISCONNECT
    ImplSpecificError = 0x83, // CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    NotAuthorized = 0x87, // CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    ServerBusy = 0x89, // CONNACK, DISCONNECT
    BadAuthMethod = 0x8C, // CONNACK, DISCONNECT
    TopicFilterInvalid = 0x8F, // SUBACK, UNSUBACK, DISCONNECT
    TopicNameInvalid = 0x90, // CONNACK, PUBACK, PUBREC, DISCONNECT
    PacketIdentifierInUse = 0x91, // PUBACK, SUBACK, UNSUBACK
    PacketIdentifierNotFound = 0x92, // PUBREL, PUBCOMP
    PacketTooLarge = 0x95, // CONNACK, PUBACK, PUBREC, DISCONNECT
    QuotaExceeded = 0x97, // PUBACK, PUBREC, SUBACK, DISCONNECT
    PayloadFormatInvalid = 0x99, // CONNACK, DISCONNECT
    RetainNotSupported = 0x9A, // CONNACK, DISCONNECT
    QoSNotSupported = 0x9B, // CONNACK, DISCONNECT
    UseAnotherServer = 0x9C, // CONNACK, DISCONNECT
    ServerMoved = 0x9D, // CONNACK, DISCONNECT
    SharedSubscriptionsNotSupported = 0x9E, // SUBACK, DISCONNECT
    ConnectionRateExceeded = 0x9F, // CONNACK, DISCONNECT
    SubscriptionIdsNotSupported = 0xA1, // SUBACK, DISCONNECT
    WildcardSubscriptionsNotSupported = 0xA2, // SUBACK, DISCONNECT
}

impl ReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasonCode::Success => "Success",
            ReasonCode::NoMatchingSubscribers => "No matching subscribers",
            ReasonCode::UnspecifiedError => "Unspecified error",
            ReasonCode::MalformedPacket => "Malformed Packet",
            ReasonCode::ProtocolError => "Protocol Error",
            ReasonCode::ImplSpecificError => "Implementation specific error",
            ReasonCode::NotAuthorized => "Not authorized",
            ReasonCode::ServerBusy => "Server busy",
            ReasonCode::BadAuthMethod => "Bad authentication method",
            ReasonCode::TopicFilterInvalid => "Topic Filter invalid",
            ReasonCode::TopicNameInvalid => "Topic Name invalid",
            ReasonCode::PacketIdentifierInUse => "Packet Identifier in use",
            ReasonCode::PacketIdentifierNotFound => "Packet Identifier not found",
            ReasonCode::PacketTooLarge => "Packet too large",
            ReasonCode::QuotaExceeded => "Quota exceeded",
            ReasonCode::PayloadFormatInvalid => "Payload format invalid",
            ReasonCode::RetainNotSupported => "Retain not supported",
            ReasonCode::QoSNotSupported => "QoS not supported",
            ReasonCode::UseAnotherServer => "Use another server",
            ReasonCode::ServerMoved => "Server moved",
            ReasonCode::SharedSubscriptionsNotSupported => "Shared Subscriptions not supported",
            ReasonCode::ConnectionRateExceeded => "Connection rate exceeded",
            ReasonCode::SubscriptionIdsNotSupported => "Subscription Identifiers not supported",
            ReasonCode::WildcardSubscriptionsNotSupported => "Wildcard Subscriptions not supported",
        }
    }
}

pub struct FixedHeaderReader {}

impl FixedHeaderReader {
    pub fn read<R: Reader>(r: &mut R) -> Result<(u8, u32), Error> {
        let byte0: u8 = r.read_u8()?;
        let remaining_len: u32 = r.read_varuint32()?;
        return Ok((byte0, remaining_len));
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::FixedHeaderReader;

    #[test]
    fn test_fixed_header_read() {
        let mut cur: Cursor<Vec<u8>> = Cursor::new(vec![0x10, 0x1B, 0x00, 0x04]);
        let result = FixedHeaderReader::read(&mut cur);
        assert!(
            result.is_ok(),
            "Error reading fixed header {}",
            result.unwrap_err()
        );
        let hdr = result.unwrap();
        assert_eq!(hdr.0, 0x10);
        assert_eq!(hdr.1, 0x1B);
    }
}
