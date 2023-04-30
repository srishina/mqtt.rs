use std::{rc::Rc, sync::RwLock};

use crate::{
    errors::{PublishTopicValidationError, SubscribeTopicValidationError},
    trie::Trie,
};

pub fn validate_publish_topic(topic: &str) -> Result<(), PublishTopicValidationError> {
    if topic.len() > 65535 {
        return Err(PublishTopicValidationError::TopicLenTooLong);
    }

    if let Some(_) = topic.chars().find(|c| *c == '#' || *c == '+') {
        return Err(PublishTopicValidationError::InvalidTopic);
    }

    return Ok(());
}

pub fn validate_subscribe_topic(topic: &str) -> Result<(), SubscribeTopicValidationError> {
    if topic.is_empty() {
        return Err(SubscribeTopicValidationError::EmptySubscriptionTopic);
    }

    if topic.len() > 65535 {
        return Err(SubscribeTopicValidationError::TopicLenTooLong);
    }

    let mut previous_char: char = topic.chars().nth(0).unwrap();
    let topic_len = topic.len();

    for (i, c) in topic.chars().enumerate() {
        if c == '+' {
            if (i != 0 && previous_char != '/')
                || (i < topic_len - 1 && topic.chars().nth(i + 1).unwrap() != '/')
            {
                return Err(SubscribeTopicValidationError::InvalidTopic(c));
            }
        } else if c == '#' {
            if (i != 0 && previous_char != '/') || (i < (topic_len - 1)) {
                return Err(SubscribeTopicValidationError::InvalidTopic(c));
            }
        }
        previous_char = c;
    }

    return Ok(());
}

pub struct TopicMatcher {
    trie: RwLock<Rc<Trie>>,
}

impl TopicMatcher {
    pub fn new() -> Self {
        Self {
            trie: RwLock::new(Rc::new(Trie::new())),
        }
    }

    pub fn subscribe(&self, topic: &str) -> Result<(), SubscribeTopicValidationError> {
        let trie = self.trie.write().unwrap();
        let result = validate_subscribe_topic(topic);
        match result {
            Ok(_v) => {
                trie.insert(topic);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn unsubscribe(&self, topic: &str) {
        let trie = self.trie.write().unwrap();
        trie.delete(topic)
    }

    pub fn match_topic(&self, topic: &str) -> bool {
        let trie = self.trie.read().unwrap();
        return trie.contains(topic);
    }

    pub fn number_of_subscriptions(&self) {
        let trie = self.trie.read().unwrap();
        trie.number_of_entries();
    }

    pub fn print_subscriptions(&self) {
        let trie = self.trie.read().unwrap();
        trie.print_entries();
    }
}

#[cfg(test)]
mod tests {
    use super::validate_publish_topic;
    use super::validate_subscribe_topic;
    use super::TopicMatcher;

    #[test]
    fn test_basic() {
        let validated = validate_publish_topic("a/b/c/d");
        assert!(validated.is_ok(), "{}", validated.unwrap_err());
    }

    #[test]
    fn test_publish_topic_validation() {
        let valid_publish_topics = ["pub/topic", "pub//topic", "pub/ /topic"];
        for t in valid_publish_topics {
            let result = validate_publish_topic(t);
            assert!(result.is_ok(), "{}", result.unwrap_err());
        }

        let invalid_publish_topics = [
            "+pub/topic",
            "pub+/topic",
            "pub/+topic",
            "pub/topic+",
            "pub/topic/+",
            "#pub/topic",
            "pub#/topic",
            "pub/#topic",
            "pub/topic#",
            "pub/topic/#",
            "+/pub/topic",
        ];
        for t in invalid_publish_topics {
            let result = validate_publish_topic(t);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_subscribe_topic_validation() {
        let valid_subscribe_topics = [
            "sub/topic",
            "sub//topic",
            "sub/ /topic",
            "sub/+/topic",
            "+/+/+",
            "+",
            "sub/topic/#",
            "sub//topic/#",
            "sub/ /topic/#",
            "sub/+/topic/#",
            "+/+/+/#",
            "#",
            "/#",
            "sub/topic/+/#",
        ];
        for t in valid_subscribe_topics {
            let result = validate_subscribe_topic(t);
            assert!(
                result.is_ok(),
                "Validation of topic {} failed. Error: {}",
                t,
                result.unwrap_err()
            );
        }

        let invalid_subscribe_topics = [
            "+sub/topic",
            "sub+/topic",
            "sub/+topic",
            "sub/topic+",
            "#sub/topic",
            "sub#/topic",
            "sub/#topic",
            "sub/topic#",
            "#/sub/topic",
            "",
        ];
        for t in invalid_subscribe_topics {
            let result = validate_subscribe_topic(t);
            assert!(result.is_err(), "Invalid topic '{}' is validated.", t,);
        }
    }

    #[test]
    fn test_subscribe_valid_topic_match() {
        let valid_subscribe_topic_matches = [
            ("foo/#", "foo"),
            ("foo//bar", "foo//bar"),
            ("foo//+", "foo//bar"),
            ("foo/+/+/baz", "foo///baz"),
            ("foo/bar/+", "foo/bar/"),
            ("foo/bar", "foo/bar"),
            ("foo/+", "foo/bar"),
            ("foo/+/baz", "foo/bar/baz"),
            ("A/B/+/#", "A/B/B/C"),
            ("foo/+/#", "foo/bar"),
            ("#", "foo/bar/baz"),
            ("/#", "/foo/bar"),
            ("foo/+/#", "foo/bar/baz"),
            ("#", "foo/bar/baz"),
            ("/#", "/foo/bar"),
        ];
        for t in valid_subscribe_topic_matches {
            let matcher = TopicMatcher::new();
            let result = matcher.subscribe(t.0);
            assert!(
                result.is_ok(),
                "Error subscribing the topic'{}', Error: {}",
                t.0,
                result.unwrap_err()
            );
            assert!(
                matcher.match_topic(t.1),
                "Matching of topic '{}' with '{}' failed, should match",
                t.1,
                t.0,
            );
        }
    }

    #[test]
    fn test_subscribe_valid_topic_no_match() {
        let valid_subscribe_topic_no_matches = [
            ("test/6/#", "test/3"),
            ("test/6/#", "test/3"),
            ("test/6/#", "test/3"),
            ("test/6/#", "test/^^3"),
            ("foo/bar", "foo"),
            ("foo/+", "foo/bar/baz"),
            ("foo/+/baz", "foo/bar/bar"),
            ("foo/+/#", "fo2/bar/baz"),
            ("/#", "foo/bar"),
            ("+foo", "+foo"),
            ("fo+o", "fo+o"),
            ("foo+", "foo+"),
            ("+foo/bar", "+foo/bar"),
            ("foo+/bar", "foo+/bar"),
            ("foo/+bar", "foo/+bar"),
            ("foo/bar+", "foo/bar+"),
            ("+foo", "afoo"),
            ("fo+o", "foao"),
            ("foo+", "fooa"),
            ("+foo/bar", "afoo/bar"),
            ("foo+/bar", "fooa/bar"),
            ("foo/+bar", "foo/abar"),
            ("foo/bar+", "foo/bara"),
            ("#foo", "#foo"),
            ("fo#o", "fo#o"),
            ("foo#", "foo#"),
            ("#foo/bar", "#foo/bar"),
            ("foo#/bar", "foo#/bar"),
            ("foo/#bar", "foo/#bar"),
            ("foo/bar#", "foo/bar#"),
            ("foo+", "fooa"),
        ];
        for t in valid_subscribe_topic_no_matches {
            let matcher = TopicMatcher::new();
            _ = matcher.subscribe(t.0);
            assert!(
                !matcher.match_topic(t.1),
                "Matching of topic '{}' with '{}' failed, should not match",
                t.1,
                t.0,
            );
        }
    }
}
