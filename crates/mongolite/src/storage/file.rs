use std::collections::HashSet;
use std::fs::{File as StdFile, OpenOptions};
use std::path::Path;

#[cfg(feature = "mmap")]
use memmap2::MmapMut;

use crate::error::{Error, Result};

pub const MAGIC: [u8; 4] = [0x4D, 0x44, 0x4F, 0x43]; // "MDOC"
pub const DEFAULT_PAGE_SIZE: u32 = 4096;
pub const VERSION_MAJOR: u16 = 0;
pub const VERSION_MINOR: u16 = 1;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    pub magic: [u8; 4],
    pub version_major: u16,
    pub version_minor: u16,
    pub page_size: u32,
    pub total_pages: u32,
    pub page_count_at_checkpoint: u32,
    pub free_list_head: u32,
    pub catalog_root_page: u32,
    pub wal_magic: u32,
    pub document_count: u64,
    pub checksum: u32,
}

impl FileHeader {
    pub const SIZE: usize = 4 + 2 + 2 + 4 + 4 + 4 + 4 + 4 + 4 + 8 + 4;

    pub fn new(page_size: u32, total_pages: u32) -> Self {
        Self {
            magic: MAGIC,
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            page_size,
            total_pages,
            page_count_at_checkpoint: 1,
            free_list_head: 0,
            catalog_root_page: 0,
            wal_magic: 0,
            document_count: 0,
            checksum: 0,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::SIZE);
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.version_major.to_le_bytes());
        buf.extend_from_slice(&self.version_minor.to_le_bytes());
        buf.extend_from_slice(&self.page_size.to_le_bytes());
        buf.extend_from_slice(&self.total_pages.to_le_bytes());
        buf.extend_from_slice(&self.page_count_at_checkpoint.to_le_bytes());
        buf.extend_from_slice(&self.free_list_head.to_le_bytes());
        buf.extend_from_slice(&self.catalog_root_page.to_le_bytes());
        buf.extend_from_slice(&self.wal_magic.to_le_bytes());
        buf.extend_from_slice(&self.document_count.to_le_bytes());
        buf.extend_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::Corrupted("header too short".into()));
        }

        let mut offset = 0;
        let magic = data[offset..offset + 4].try_into().unwrap();
        offset += 4;

        let version_major = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let version_minor = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
        offset += 2;

        let page_size = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let total_pages = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let page_count_at_checkpoint = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let free_list_head = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let catalog_root_page = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let wal_magic = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let document_count = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let checksum = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());

        Ok(Self {
            magic,
            version_major,
            version_minor,
            page_size,
            total_pages,
            page_count_at_checkpoint,
            free_list_head,
            catalog_root_page,
            wal_magic,
            document_count,
            checksum,
        })
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut copy = *self;
        copy.checksum = 0;
        let bytes = copy.to_bytes();
        crc32fast::hash(&bytes[..bytes.len() - 4])
    }

    pub fn validate(data: &[u8]) -> Result<()> {
        let header = Self::from_bytes(data)?;

        if header.magic != MAGIC {
            return Err(Error::Corrupted("invalid magic".into()));
        }

        if header.version_major != VERSION_MAJOR {
            let major = header.version_major;
            let minor = header.version_minor;
            return Err(Error::Corrupted(format!(
                "unsupported version: {}.{}",
                major, minor
            )));
        }

        let stored_checksum = header.checksum;
        let computed = header.compute_checksum();

        if stored_checksum != computed {
            return Err(Error::Corrupted("header checksum mismatch".into()));
        }

        Ok(())
    }
}

pub struct File {
    file: StdFile,
    mmap: MmapMut,
    page_size: u32,
    dirty_pages: HashSet<u32>,
}

impl File {
    pub fn create(path: &Path, page_size: u32) -> Result<Self> {
        if page_size < 512 || !page_size.is_power_of_two() {
            return Err(Error::InvalidPageSize(page_size));
        }

        let total_pages = 1u32;
        let file_size = page_size as usize;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;

        file.set_len(file_size as u64)?;

        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        let mut header = FileHeader::new(page_size, total_pages);
        header.checksum = header.compute_checksum();
        let header_bytes = header.to_bytes();

        mmap[..header_bytes.len()].copy_from_slice(&header_bytes);
        mmap[header_bytes.len()..].fill(0);

        Ok(Self {
            file,
            mmap,
            page_size,
            dirty_pages: HashSet::new(),
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        if mmap.len() < FileHeader::SIZE {
            return Err(Error::Corrupted("file too small for header".into()));
        }

        FileHeader::validate(&mmap[..FileHeader::SIZE])?;

        let header = FileHeader::from_bytes(&mmap[..FileHeader::SIZE])?;
        let page_size = header.page_size;

        if page_size < 512 || !page_size.is_power_of_two() {
            return Err(Error::InvalidPageSize(page_size));
        }

        let expected_size = page_size as usize * header.total_pages as usize;
        if mmap.len() < expected_size {
            return Err(Error::Corrupted(format!(
                "file size {} is less than expected {}",
                mmap.len(),
                expected_size
            )));
        }

        Ok(Self {
            file,
            mmap,
            page_size,
            dirty_pages: HashSet::new(),
        })
    }

    pub fn open_raw(path: &Path, page_size: u32) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        if mmap.len() < page_size as usize {
            return Err(Error::Corrupted("file too small for page size".into()));
        }

        Ok(Self {
            file,
            mmap,
            page_size,
            dirty_pages: HashSet::new(),
        })
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn total_pages(&self) -> u32 {
        self.mmap.len() as u32 / self.page_size
    }

    pub fn page(&self, page_id: u32) -> &[u8] {
        let start = page_id as usize * self.page_size as usize;
        let end = start + self.page_size as usize;
        &self.mmap[start..end]
    }

    pub fn page_mut(&mut self, page_id: u32) -> &mut [u8] {
        let start = page_id as usize * self.page_size as usize;
        let end = start + self.page_size as usize;
        self.dirty_pages.insert(page_id);
        &mut self.mmap[start..end]
    }

    pub fn grow(&mut self) -> Result<()> {
        let current_total = self.total_pages();
        let new_total = current_total * 2;
        let new_size = new_total as usize * self.page_size as usize;

        self.mmap.flush()?;
        self.file.set_len(new_size as u64)?;
        self.mmap = unsafe { MmapMut::map_mut(&self.file)? };

        self.dirty_pages.extend(0..new_total);

        let header = self.header_mut();
        header.total_pages = new_total;
        header.checksum = header.compute_checksum();

        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.mmap.flush()?;
        self.dirty_pages.clear();
        Ok(())
    }

    pub fn header(&self) -> FileHeader {
        FileHeader::from_bytes(&self.mmap[..FileHeader::SIZE]).unwrap()
    }

    pub fn header_mut(&mut self) -> &mut FileHeader {
        self.dirty_pages.insert(0);
        unsafe {
            let ptr = self.mmap.as_mut_ptr() as *mut FileHeader;
            &mut *ptr
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        assert_eq!(file.page_size(), DEFAULT_PAGE_SIZE);
        assert_eq!(file.total_pages(), 1);

        let header = file.header();
        let magic = header.magic;
        let version_major = header.version_major;
        let version_minor = header.version_minor;
        assert_eq!(magic, MAGIC);
        assert_eq!(version_major, VERSION_MAJOR);
        assert_eq!(version_minor, VERSION_MINOR);
    }

    #[test]
    fn test_open_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let _file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        }

        let file = File::open(&path).unwrap();
        assert_eq!(file.page_size(), DEFAULT_PAGE_SIZE);
        assert_eq!(file.total_pages(), 1);
    }

    #[test]
    fn test_open_invalid_magic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
            let header = file.header_mut();
            header.magic = [0x00; 4];
            header.checksum = header.compute_checksum();
            file.flush().unwrap();
        }

        let result = File::open(&path);
        assert!(matches!(result, Err(Error::Corrupted(_))));
    }

    #[test]
    fn test_open_invalid_checksum() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
            let header = file.header_mut();
            header.checksum = 0xDEADBEEF;
            file.flush().unwrap();
        }

        let result = File::open(&path);
        assert!(matches!(result, Err(Error::Corrupted(_))));
    }

    #[test]
    fn test_page_access() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let page = file.page(0);
        assert_eq!(page.len(), DEFAULT_PAGE_SIZE as usize);

        let page_mut = file.page_mut(0);
        page_mut[0] = 0x42;
        assert_eq!(file.page(0)[0], 0x42);
    }

    #[test]
    fn test_flush() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
            let page = file.page_mut(0);
            page[FileHeader::SIZE] = 0xAB;
            file.flush().unwrap();
        }

        let file = File::open(&path).unwrap();
        assert_eq!(file.page(0)[FileHeader::SIZE], 0xAB);
    }

    #[test]
    fn test_invalid_page_size() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let result = File::create(&path, 100);
        assert!(matches!(result, Err(Error::InvalidPageSize(100))));
    }

    #[test]
    fn test_header_roundtrip() {
        let mut header = FileHeader::new(4096, 10);
        header.free_list_head = 5;
        header.catalog_root_page = 3;
        header.document_count = 42;
        header.checksum = header.compute_checksum();

        let bytes = header.to_bytes();
        let parsed = FileHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, header.magic);
        let parsed_version_major = parsed.version_major;
        let header_version_major = header.version_major;
        assert_eq!(parsed_version_major, header_version_major);
        let parsed_version_minor = parsed.version_minor;
        let header_version_minor = header.version_minor;
        assert_eq!(parsed_version_minor, header_version_minor);
        let parsed_page_size = parsed.page_size;
        let header_page_size = header.page_size;
        assert_eq!(parsed_page_size, header_page_size);
        let parsed_total_pages = parsed.total_pages;
        let header_total_pages = header.total_pages;
        assert_eq!(parsed_total_pages, header_total_pages);
        let parsed_free_list_head = parsed.free_list_head;
        let header_free_list_head = header.free_list_head;
        assert_eq!(parsed_free_list_head, header_free_list_head);
        let parsed_catalog_root_page = parsed.catalog_root_page;
        let header_catalog_root_page = header.catalog_root_page;
        assert_eq!(parsed_catalog_root_page, header_catalog_root_page);
        let parsed_document_count = parsed.document_count;
        let header_document_count = header.document_count;
        assert_eq!(parsed_document_count, header_document_count);
        let parsed_checksum = parsed.checksum;
        let header_checksum = header.checksum;
        assert_eq!(parsed_checksum, header_checksum);
    }

    #[test]
    fn test_grow() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        assert_eq!(file.total_pages(), 1);

        file.grow().unwrap();
        assert_eq!(file.total_pages(), 2);

        file.grow().unwrap();
        assert_eq!(file.total_pages(), 4);
    }
}
