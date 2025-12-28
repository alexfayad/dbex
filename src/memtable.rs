use std::collections::BTreeMap;

pub struct MemTable {
    data: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    size_bytes: usize,  // Track size
}

impl MemTable {
    pub fn new() -> Self {
        MemTable{
            data: BTreeMap::new(),
            size_bytes: 0,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if let Some(Some(old_value)) = self.data.get(&key) {
            self.size_bytes -= key.len() + old_value.len();
        }

        self.size_bytes += key.len() + value.len();
        self.data.insert(key, Some(value));
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        match self.data.get(key)? {
            Some(value) => Some(value),
            None => None,
        }
    }

    pub fn remove(&mut self, key: &[u8]) {
        self.data.insert(key.to_vec(), None);  // Tombstone
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> BTreeMap<Vec<u8>, Option<Vec<u8>>> {
        self.data.clone()
    }

    pub fn size_byte(&self) -> usize {
        self.size_bytes
    }

    pub fn copy(&self) -> MemTable {
        MemTable{
            data: self.data.clone(),
            size_bytes: self.size_bytes.clone(),
        }
    }
}