//! 一致性哈希（spec §4.7）
//!
//! 使用虚拟节点减少数据倾斜。

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::core::util::hash::sha256;

const DEFAULT_VIRTUAL_NODES: usize = 150;

/// 一致性哈希环
pub struct ConsistentHash<T: Clone + Hash + Eq> {
    ring: BTreeMap<u64, T>,
    #[allow(dead_code)]
    virtual_nodes: usize,
}

impl<T: Clone + Hash + Eq> ConsistentHash<T> {
    pub fn new(buckets: &[T]) -> Self {
        Self::with_virtual_nodes(buckets, DEFAULT_VIRTUAL_NODES)
    }

    pub fn with_virtual_nodes(buckets: &[T], virtual_nodes: usize) -> Self {
        let mut ring = BTreeMap::new();
        for bucket in buckets {
            for i in 0..virtual_nodes {
                let key = hash_bucket_node(bucket, i);
                ring.insert(key, bucket.clone());
            }
        }
        Self {
            ring,
            virtual_nodes,
        }
    }

    /// 获取 key 对应的桶
    pub fn get(&self, key: &str) -> Option<&T> {
        if self.ring.is_empty() {
            return None;
        }
        let h = hash_key(key);
        // 找到第一个 >= h 的节点
        let result = self.ring.range(h..).next();
        match result {
            Some((_, v)) => Some(v),
            None => self.ring.values().next(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}

fn hash_key(key: &str) -> u64 {
    let digest = sha256(key.as_bytes());
    let mut v: u64 = 0;
    for (i, b) in digest.iter().take(8).enumerate() {
        v |= (*b as u64) << (i * 8);
    }
    v
}

fn hash_bucket_node<T: Hash>(bucket: &T, index: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bucket.hash(&mut hasher);
    index.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_bucket_for_any_key() {
        let buckets = vec!["a", "b", "c"];
        let ring = ConsistentHash::new(&buckets);
        assert!(ring.get("any-key").is_some());
    }

    #[test]
    fn empty_buckets_returns_none() {
        let ring: ConsistentHash<String> = ConsistentHash::new(&[]);
        assert!(ring.get("key").is_none());
    }

    #[test]
    fn distribution_reasonably_even() {
        let buckets: Vec<String> = (0..5).map(|i| format!("bucket-{i}")).collect();
        let ring = ConsistentHash::new(&buckets);
        let mut counts = std::collections::HashMap::new();
        for i in 0..1000 {
            let key = format!("key-{i}");
            let b = ring.get(&key).unwrap();
            *counts.entry(b.clone()).or_insert(0u32) += 1;
        }
        // 每个桶至少分到 50 次（1000/5=200 期望，5% 下限）
        for count in counts.values() {
            assert!(*count > 50, "uneven distribution: {counts:?}");
        }
    }

    #[test]
    fn same_key_same_bucket() {
        let buckets = vec!["x", "y", "z"];
        let ring = ConsistentHash::new(&buckets);
        let a = ring.get("stable-key");
        let b = ring.get("stable-key");
        assert_eq!(a, b);
    }
}
