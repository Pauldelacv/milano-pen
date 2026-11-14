use std::collections::HashMap;
use std::sync::Arc;

/// A least-recently-used cache bounded by total bytes, for hot thumbnails.
pub struct ByteLru {
    map: HashMap<String, (Arc<Vec<u8>>, u64)>,
    tick: u64,
    bytes: usize,
    capacity: usize,
}

impl ByteLru {
    pub fn new(capacity_bytes: usize) -> Self {
        Self { map: HashMap::new(), tick: 0, bytes: 0, capacity: capacity_bytes }
    }

    pub fn get(&mut self, key: &str) -> Option<Arc<Vec<u8>>> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(key).map(|(v, t)| {
            *t = tick;
            v.clone()
        })
    }

    pub fn insert(&mut self, key: String, value: Arc<Vec<u8>>) {
        if value.len() > self.capacity / 4 {
            return; // too big to be worth caching
        }
        self.tick += 1;
        if let Some((old, _)) = self.map.insert(key, (value.clone(), self.tick)) {
            self.bytes -= old.len();
        }
        self.bytes += value.len();
        while self.bytes > self.capacity {
            let Some(oldest) = self.map.iter().min_by_key(|(_, (_, t))| *t).map(|(k, _)| k.clone()) else {
                break;
            };
            if let Some((v, _)) = self.map.remove(&oldest) {
                self.bytes -= v.len();
            }
        }
    }

    pub fn remove_prefix(&mut self, prefix: &str) {
        let keys: Vec<String> = self.map.keys().filter(|k| k.starts_with(prefix)).cloned().collect();
        for k in keys {
            if let Some((v, _)) = self.map.remove(&k) {
                self.bytes -= v.len();
            }
        }
    }

    pub fn len_bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used_first() {
        let mut c = ByteLru::new(100);
        c.insert("a".into(), Arc::new(vec![0; 20]));
        c.insert("b".into(), Arc::new(vec![0; 20]));
        c.insert("c".into(), Arc::new(vec![0; 20]));
        assert!(c.get("a").is_some()); // a is now the most recent
        c.insert("d".into(), Arc::new(vec![0; 20]));
        c.insert("e".into(), Arc::new(vec![0; 20])); // 100 bytes: still fits
        c.insert("f".into(), Arc::new(vec![0; 20])); // evicts b
        assert!(c.get("b").is_none());
        assert!(c.get("a").is_some());
        assert!(c.len_bytes() <= 100);
    }

    #[test]
    fn skips_oversized_values_and_replaces_keys() {
        let mut c = ByteLru::new(100);
        c.insert("big".into(), Arc::new(vec![0; 60]));
        assert!(c.is_empty());
        c.insert("k".into(), Arc::new(vec![0; 10]));
        c.insert("k".into(), Arc::new(vec![0; 5]));
        assert_eq!(c.len_bytes(), 5);
        c.remove_prefix("k");
        assert_eq!(c.len(), 0);
    }
}
