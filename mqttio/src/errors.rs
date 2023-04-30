#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid or malformed utf-8 string")]
    InvalidUTF8String,
    #[error("malformed packet")]
    MalformedPacket,
    #[error("variable integer contained more than maximum bytes ({0})")]
    InvalidVarUint32(u32),
    #[error("variable integer contains value of {0} which is more than the permissible")]
    InvalidVarUint32Length(u32),
    #[error("{0} property must not be included more than once")]
    PropertyAlreadyExists(&'static str),
    #[error("invalid property id - Malformed packet")]
    InvalidPropertyID(u32),
}
