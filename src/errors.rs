#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("topic length is too long, max = 65335")]
    TopicLenTooLong,
    #[error("invalid topic")]
    InvalidTopic,
    #[error("empty subscription topics are not allowed")]
    EmptySubscriptionTopic,
    #[error("{0} property must not be included more than once")]
    PropertyAlreadyExists(&'static str),
    #[error("invalid protocol name - found {0}")]
    InvalidProtocolName(String),
    #[error("invalid protocol version - only version 5 is supported")]
    InvalidProtocolVersion,
    #[error("invalid connect flags - Malformed packet")]
    InvalidConnectFlags,
    #[error("invalid Will QoS - Malformed packet")]
    InvalidWillQos,
    #[error("invalid Will QoS flags - Malformed packet")]
    InvalidQosFlags,
    #[error("invalid Will retain flag - Malformed packet")]
    InvalidWillRetain,
    #[error("invalid property id - Malformed packet")]
    InvalidPropertyID(u32),
    #[error("CONNECT - Will properties contains wrong property identifier {0}")]
    InvalidWillPropertyID(u32),
    #[error(transparent)]
    IOError(#[from] mqttio::errors::Error),
    #[error("Invalid remaining length")]
    InvalidRemaningLength(core::num::TryFromIntError),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PublishTopicValidationError {
    #[error("publish {}", Error::TopicLenTooLong)]
    TopicLenTooLong,
    #[error("{} - publish topic cannot contain '+' or '*'", Error::InvalidTopic)]
    InvalidTopic,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SubscribeTopicValidationError {
    #[error("subscription {}", Error::TopicLenTooLong)]
    TopicLenTooLong,
    #[error("{}", Error::EmptySubscriptionTopic)]
    EmptySubscriptionTopic,
    #[error(
        "{} - subscribe topic cannot contain the character '{0}'",
        Error::InvalidTopic
    )]
    InvalidTopic(char),
}
