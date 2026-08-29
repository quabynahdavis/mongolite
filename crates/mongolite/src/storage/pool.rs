use std::collections::{HashMap, VecDeque};

use crate::error::Result;
use crate::storage::file::File;

pub struct PagePool {
    cache: HashMap<u32, Vec<u8>>,
    lru: VecDeque<u32>,
    capacity: usize,
}

impl PagePool {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            lru: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get(&mut self, file: &File, page_id: u32) -> Result<&[u8]> {
        if !self.cache.contains_key(&page_id) {
            self.load_page(file, page_id)?;
        }

        if let Some(pos) = self.lru.iter().position(|&id| id == page_id) {
            self.lru.remove(pos);
        }
        self.lru.push_back(page_id);

        Ok(self.cache.get(&page_id).map(|v| v.as_slice()).unwrap())
    }

    pub fn get_mut(&mut self, file: &File, page_id: u32) -> Result<&mut [u8]> {
        if !self.cache.contains_key(&page_id) {
            self.load_page(file, page_id)?;
        }

        if let Some(pos) = self.lru.iter().position(|&id| id == page_id) {
            self.lru.remove(pos);
        }
        self.lru.push_back(page_id);

        Ok(self
            .cache
            .get_mut(&page_id)
            .map(|v| v.as_mut_slice())
            .unwrap())
    }

    pub fn flush(&mut self, file: &mut File) -> Result<()> {
        for (page_id, data) in &self.cache {
            let page = file.page_mut(*page_id);
            page.copy_from_slice(data);
        }
        file.flush()?;
        self.cache.clear();
        self.lru.clear();
        Ok(())
    }

    pub fn put(&mut self, page_id: u32, data: Vec<u8>) {
        if let std::collections::hash_map::Entry::Occupied(mut e) = self.cache.entry(page_id) {
            e.insert(data);
            if let Some(pos) = self.lru.iter().position(|&id| id == page_id) {
                self.lru.remove(pos);
            }
            self.lru.push_back(page_id);
            return;
        }

        if self.cache.len() >= self.capacity {
            self.evict();
        }

        self.cache.insert(page_id, data);
        self.lru.push_back(page_id);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru.clear();
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    fn load_page(&mut self, file: &File, page_id: u32) -> Result<()> {
        if self.cache.len() >= self.capacity {
            self.evict();
        }

        let page_data = file.page(page_id).to_vec();
        self.cache.insert(page_id, page_data);
        self.lru.push_back(page_id);
        Ok(())
    }

    fn evict(&mut self) {
        if let Some(oldest) = self.lru.pop_front() {
            self.cache.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::file::{File, DEFAULT_PAGE_SIZE};
    use tempfile::TempDir;

    #[test]
    fn test_cache_get() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        let mut pool = PagePool::new(10);

        let page = pool.get(&file, 0).unwrap();
        assert_eq!(page.len(), DEFAULT_PAGE_SIZE as usize);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_cache_get_mut() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        let mut pool = PagePool::new(10);

        let page = pool.get_mut(&file, 0).unwrap();
        page[0] = 0x42;

        let page = pool.get(&file, 0).unwrap();
        assert_eq!(page[0], 0x42);
    }

    #[test]
    fn test_cache_eviction() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        file.grow().unwrap();
        file.grow().unwrap();

        let mut pool = PagePool::new(3);

        pool.get(&file, 0).unwrap();
        pool.get(&file, 1).unwrap();
        pool.get(&file, 2).unwrap();
        assert_eq!(pool.len(), 3);

        pool.get(&file, 3).unwrap();
        assert_eq!(pool.len(), 3);
        assert!(!pool.cache.contains_key(&0));
    }

    #[test]
    fn test_cache_lru_order() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        file.grow().unwrap();
        file.grow().unwrap();

        let mut pool = PagePool::new(3);

        pool.get(&file, 0).unwrap();
        pool.get(&file, 1).unwrap();
        pool.get(&file, 2).unwrap();

        pool.get(&file, 0).unwrap();

        pool.get(&file, 3).unwrap();
        assert!(pool.cache.contains_key(&0));
        assert!(!pool.cache.contains_key(&1));
    }

    #[test]
    fn test_cache_flush() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        let mut pool = PagePool::new(10);

        let page = pool.get_mut(&file, 0).unwrap();
        page[100] = 0xAB;
        pool.flush(&mut file).unwrap();

        assert!(pool.is_empty());

        let file = File::open(&path).unwrap();
        assert_eq!(file.page(0)[100], 0xAB);
    }

    #[test]
    fn test_cache_put() {
        let mut pool = PagePool::new(10);
        pool.put(0, vec![0x42; 64]);

        let page = pool
            .get(
                &File::create(
                    &tempfile::TempDir::new().unwrap().path().join("dummy"),
                    DEFAULT_PAGE_SIZE,
                )
                .unwrap(),
                0,
            )
            .unwrap();
        assert_eq!(page[0], 0x42);
    }

    #[test]
    fn test_cache_clear() {
        let mut pool = PagePool::new(10);
        pool.put(0, vec![0x42; 64]);
        pool.put(1, vec![0x43; 64]);

        assert_eq!(pool.len(), 2);

        pool.clear();
        assert!(pool.is_empty());
    }
}
