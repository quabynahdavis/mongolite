use crate::error::{Error, Result};
use crate::storage::file::File;
use crate::storage::page::{Page, PageHeader, PageType};

pub struct Allocator<'a> {
    file: &'a mut File,
    free_list_head: u32,
}

impl<'a> Allocator<'a> {
    pub fn new(file: &'a mut File) -> Self {
        let free_list_head = file.header().free_list_head;
        Self {
            file,
            free_list_head,
        }
    }

    pub fn allocate(&mut self) -> Result<u32> {
        if self.free_list_head == 0 {
            let total = self.file.total_pages();
            self.file.grow()?;
            return Ok(total);
        }

        let page_id = self.free_list_head;
        let page = Page::new(self.file.page(page_id))?;
        let page_data = page.data();
        let next_free = u32::from_le_bytes(page_data[..4].try_into().unwrap());

        if next_free != 0 {
            self.free_list_head = next_free;
            return Ok(page_id);
        }

        let entry_count = page_data.len() / 4;
        for i in 1..entry_count {
            let offset = i * 4;
            let id = u32::from_le_bytes(page_data[offset..offset + 4].try_into().unwrap());
            if id != 0 {
                let page_mut = self.file.page_mut(page_id);
                page_mut[PageHeader::SIZE + offset..PageHeader::SIZE + offset + 4]
                    .copy_from_slice(&0u32.to_le_bytes());
                return Ok(id);
            }
        }

        Ok(page_id)
    }

    pub fn free(&mut self, page_id: u32) -> Result<()> {
        if page_id == 0 {
            return Err(Error::Corrupted("cannot free header page".into()));
        }

        let free_list_head = self.free_list_head;
        let page_size = self.file.page_size() as usize;

        let page_mut = self.file.page_mut(page_id);
        page_mut.fill(0);

        page_mut[PageHeader::SIZE..PageHeader::SIZE + 4]
            .copy_from_slice(&free_list_head.to_le_bytes());

        let header_bytes = PageHeader {
            page_type: PageType::Free as u8,
            flags: 0,
            padding: 0,
            checksum: 0,
        }
        .to_bytes();
        page_mut[..PageHeader::SIZE].copy_from_slice(&header_bytes);

        let checksum = crc32fast::hash(&page_mut[PageHeader::SIZE..page_size]);
        page_mut[4..8].copy_from_slice(&checksum.to_le_bytes());

        self.free_list_head = page_id;

        let header = self.file.header_mut();
        header.free_list_head = page_id;

        Ok(())
    }

    pub fn free_list_head(&self) -> u32 {
        self.free_list_head
    }

    pub fn file(&self) -> &File {
        self.file
    }

    pub fn file_mut(&mut self) -> &mut File {
        self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::file::{File, DEFAULT_PAGE_SIZE};
    use tempfile::TempDir;

    #[test]
    fn test_allocate_from_free_list() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let mut allocator = Allocator::new(&mut file);
        let page1 = allocator.allocate().unwrap();
        assert!(page1 > 0);

        allocator.free(page1).unwrap();
        assert_eq!(allocator.free_list_head(), page1);

        let page2 = allocator.allocate().unwrap();
        assert_eq!(page2, page1);
    }

    #[test]
    fn test_allocate_multiple_pages() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let mut allocator = Allocator::new(&mut file);
        let page1 = allocator.allocate().unwrap();
        let page2 = allocator.allocate().unwrap();

        assert_ne!(page1, page2);
        assert!(page1 > 0);
        assert!(page2 > 0);
    }

    #[test]
    fn test_free_list_integrity() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let mut allocator = Allocator::new(&mut file);
        let pages: Vec<u32> = (0..5).map(|_| allocator.allocate().unwrap()).collect();

        for &page_id in &pages {
            allocator.free(page_id).unwrap();
        }

        let mut reallocated = Vec::new();
        for _ in 0..5 {
            reallocated.push(allocator.allocate().unwrap());
        }

        reallocated.sort();
        let mut sorted_pages = pages.clone();
        sorted_pages.sort();
        assert_eq!(reallocated, sorted_pages);
    }

    #[test]
    fn test_cannot_free_header_page() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let mut allocator = Allocator::new(&mut file);
        let result = allocator.free(0);
        assert!(matches!(result, Err(Error::Corrupted(_))));
    }

    #[test]
    fn test_grow_on_allocate() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        assert_eq!(file.total_pages(), 1);

        let mut allocator = Allocator::new(&mut file);
        let _page = allocator.allocate().unwrap();

        assert!(file.total_pages() > 1);
    }
}
