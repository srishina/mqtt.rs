use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::HashMap,
    iter::Peekable,
    rc::{Rc, Weak},
    str::Split,
};

type RcTrieNode = Rc<TrieNode>;
type TrieStack = Vec<Vec<String>>;

fn print_trie_nodes(node: &RcTrieNode) -> TrieStack {
    let mut stack: TrieStack = Vec::new();
    if !node.as_ref().borrow().children.borrow().is_empty() {
        stack.push(Vec::new());
        print_trie_node(node, &mut stack);
    }
    return stack;
}

fn print_trie_node(node: &RcTrieNode, stack: &mut TrieStack) {
    let current = stack.pop().unwrap();

    let borrowed_node = node.as_ref().borrow();
    let borrowed_hash_map = borrowed_node.children.borrow();
    if borrowed_hash_map.is_empty() {
        stack.push(current);
    } else {
        for (_k, v) in &*borrowed_hash_map {
            let mut child_list = current.clone();
            if let Some(data) = &v.as_ref().borrow().value {
                child_list.push(data.to_string());
            }

            if v.has_subscription() && v.has_children() {
                stack.push(child_list.clone());
            }

            stack.push(child_list);

            print_trie_node(v, stack);
        }
    }
}

fn match_topic_part(
    node: &RcTrieNode,
    parts: &mut Peekable<Split<&str>>,
    current: Option<&str>,
) -> bool {
    fn match_child(node: &RcTrieNode, parts: &mut Peekable<Split<&str>>, value: &str) -> bool {
        let child = node.get_child(value);
        match child {
            Some(v) => {
                // found +, check it is the last part
                // from MQTTv5 spec
                // e.g “sport/tennis/+” matches “sport/tennis/player1” and
                // “sport/tennis/player2”, but not “sport/tennis/player1/ranking”.
                if value == "+" && parts.peek().is_none() {
                    return true;
                }
                let next = parts.next();
                match_topic_part(&v, parts, next)
            }
            _ => false,
        }
    }

    // "foo/#” also matches the singular "foo", since # includes the parent
    // level.
    if node.has_child("#") {
        return true;
    }

    if current.is_none() {
        return node.has_subscription();
    }

    // the single-level wildcard matches only a single level, “sport/+” does not
    // match “sport” but it does match “sport/”.
    if match_child(node, parts, "+") {
        return true;
    }

    match current {
        Some(v) => {
            if match_child(node, parts, v) {
                return true;
            }
        }
        _ => {
            // no more element present
        }
    }
    return false;
}

fn match_topic(node: &RcTrieNode, topic: &str) -> bool {
    let mut peekable = topic.split("/").peekable();
    let parts = peekable.borrow_mut();

    let part = parts.next();
    return match_topic_part(node, parts, part);
}

#[derive(Debug)]
struct TrieNode {
    value: Option<String>,
    parent: RefCell<Weak<TrieNode>>,
    children: RefCell<HashMap<String, RcTrieNode>>,
    subscribed: RefCell<bool>,
}

fn new_parent(parent: Option<Rc<TrieNode>>) -> RefCell<Weak<TrieNode>> {
    match parent {
        Some(v) => RefCell::new(Rc::downgrade(&v)),
        _ => RefCell::new(Weak::new()),
    }
}

impl TrieNode {
    fn new(value: Option<String>, parent: Option<Rc<TrieNode>>, subscribed: bool) -> RcTrieNode {
        return Rc::new(Self {
            value: value,
            parent: new_parent(parent),
            children: RefCell::new(HashMap::new()),
            subscribed: RefCell::new(subscribed),
        });
    }

    fn has_subscription(&self) -> bool {
        return *self.subscribed.borrow();
    }

    fn set_subscription(&self, subscribed: bool) {
        *self.subscribed.borrow_mut() = subscribed;
    }

    fn get_parent(&self) -> Option<Rc<TrieNode>> {
        self.parent.borrow().upgrade()
    }

    fn has_children(&self) -> bool {
        return !self.children.borrow().is_empty();
    }

    fn has_child(&self, part: &str) -> bool {
        self.children.borrow().contains_key(part)
    }

    fn get_child(&self, part: &str) -> Option<RcTrieNode> {
        match self.children.borrow().get(part) {
            Some(v) => Some(v.clone()),
            _ => None,
        }
    }

    fn get_or_insert_child(&self, part: &str, parent: RcTrieNode, subscribed: bool) -> RcTrieNode {
        let mut map = self.children.borrow_mut();
        return map
            .entry(part.to_string())
            .or_insert_with(|| TrieNode::new(Some(part.to_string()), Some(parent), subscribed))
            .clone();
    }

    fn remove_child(&self, key: &String) {
        self.children.borrow_mut().remove(key);
    }
}

pub struct Trie {
    root: RcTrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(None, None, false),
        }
    }

    pub fn insert(&self, topic: &str) {
        let mut current_node = self.root.clone();
        let mut peekable = topic.split("/").peekable();
        let parts = peekable.borrow_mut();

        while let Some(part) = parts.next() {
            let parent = current_node.clone();
            let inserted = current_node.as_ref().borrow().get_or_insert_child(
                part,
                parent,
                parts.peek().is_none(),
            );
            current_node = inserted;
        }
    }

    pub fn delete(&self, topic: &str) {
        fn detach_child(node: &RcTrieNode) {
            let borrowed_node = node.as_ref().borrow();
            if borrowed_node.has_children() {
                if node.has_subscription() {
                    borrowed_node.set_subscription(false);
                }
                return;
            }

            let parent = borrowed_node.get_parent();
            if parent.is_none() {
                return;
            }

            if node.has_subscription() {
                let parent_node = parent.as_ref().unwrap();
                parent_node.remove_child(borrowed_node.value.as_ref().unwrap());
                detach_child(parent_node);
            }
        }

        let mut current_node = self.root.clone();
        let parts = topic.split("/");
        for part in parts {
            let child = current_node.as_ref().borrow().get_child(part);
            if child.is_none() {
                return;
            }
            current_node = child.unwrap().clone();
        }
        detach_child(&current_node);
    }

    pub fn contains(&self, topic: &str) -> bool {
        return match_topic(&self.root, topic);
    }

    pub fn number_of_entries(&self) -> usize {
        let stack = print_trie_nodes(&self.root);
        return stack.len();
    }

    pub fn print_entries(&self) {
        let stack = print_trie_nodes(&self.root);
        for v in stack {
            println!("{}", v.join("/"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Trie;

    #[test]
    fn test_basic() {
        let trie = Trie::new();
        trie.insert("a/b/c/d");
        trie.insert("a/b/c/d/x");
        trie.insert("f/g/h");
        trie.insert("i/j/k");
        assert_eq!(trie.number_of_entries(), 4);
        trie.delete("a/b/c/d");
        assert_eq!(trie.number_of_entries(), 3);
    }
}
