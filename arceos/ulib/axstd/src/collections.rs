use core::hash::{Hash, Hasher};

#[cfg(feature = "alloc")]
use alloc::{vec::Vec, boxed::Box, vec};

const INITIAL_CAPACITY: usize = 16;

pub struct HashMap<K, V> {
    buckets: Vec<Option<Box<Node<K, V>>>>,
    len: usize,
    hasher: MyHasher
}

#[derive(Clone)]
pub struct MyHasher {
    hash: u64,
    seed: u64,
}

impl MyHasher {
    pub fn new(seed: u64) -> Self {
        MyHasher { seed, hash: 0 }
    }
}

impl Hasher for MyHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.hash = self.hash.wrapping_mul(31).wrapping_add(b as u64 ^ self.seed);
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

#[cfg(feature = "alloc")]
struct Node<K, V> {
    key: K,
    value: V,
    next: Option<Box<Node<K, V>>>,
}

#[cfg(feature = "alloc")]
impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Node {
            key,
            value,
            next: None,
        }
    }
}

#[cfg(feature = "alloc")]
impl<K: Hash + Eq, V> HashMap<K, V> {
    pub fn new() -> Self {
        use arceos_api::modules::axhal::misc::random;
        let mut buckets = Vec::with_capacity(INITIAL_CAPACITY);
        buckets.resize_with(INITIAL_CAPACITY, || None);

        HashMap { 
            buckets,
            len: 0, 
            hasher: MyHasher::new(random() as u64)
        }
    }

    fn bucket_index(&self, key: &K) -> usize {
        let mut hasher = self.hasher.clone();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.buckets.len()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = self.bucket_index(&key);
        let mut current = &mut self.buckets[index];

        while let Some(node) = current {
            if node.key == key {
                let old_value = core::mem::replace(&mut node.value, value);
                return Some(old_value);
            }
            current = &mut node.next;
        }

        let new_node = Box::new(Node::new(key, value));
        self.buckets[index] = Some(new_node);
        self.len += 1;
        None
    }

    pub fn iter(&self) -> HashMapIter<K, V> {
        HashMapIter {
            buckets: &self.buckets,
            bucket_index: 0,
            current_node: None,
        }
    }
}

pub struct HashMapIter<'a, K, V> {
    buckets: &'a Vec<Option<Box<Node<K, V>>>>,
    bucket_index: usize,
    current_node: Option<&'a Node<K, V>>,
}

impl<'a, K, V> Iterator for HashMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.current_node {
            self.current_node = node.next.as_deref();
            return Some((&node.key, &node.value));
        }

        while self.bucket_index < self.buckets.len() {
            if let Some(node) = &self.buckets[self.bucket_index] {
                self.current_node = Some(node);
                self.bucket_index += 1;
                return Some((&node.key, &node.value));
            }
            self.bucket_index += 1;
        }

        None
    }
}
