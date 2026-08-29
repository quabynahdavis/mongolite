use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::storage::file::{File, DEFAULT_PAGE_SIZE};

pub const WAL_MAGIC: u32 = 0x57414C21; // "WAL!"

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct WalHeader {
    pub magic: u32,
    pub page_size: u32,
    pub last_committed_pages: u32,
    pub checksum: u32,
}

impl WalHeader {
    pub const SIZE: usize = 4 + 4 + 4 + 4;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&self.page_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.last_committed_pages.to_le_bytes());
        buf[12..16].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::Wal("WAL header too short".into()));
        }

        Ok(Self {
            magic: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            page_size: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            last_committed_pages: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            checksum: u32::from_le_bytes(data[12..16].try_into().unwrap()),
        })
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut copy = *self;
        copy.checksum = 0;
        let bytes = copy.to_bytes();
        crc32fast::hash(&bytes[..bytes.len() - 4])
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct WalRecordHeader {
    pub page_id: u32,
    pub data_length: u32,
    pub checksum: u32,
}

impl WalRecordHeader {
    pub const SIZE: usize = 4 + 4 + 4;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0..4].copy_from_slice(&self.page_id.to_le_bytes());
        buf[4..8].copy_from_slice(&self.data_length.to_le_bytes());
        buf[8..12].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::Wal("WAL record header too short".into()));
        }

        Ok(Self {
            page_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            data_length: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            checksum: u32::from_le_bytes(data[8..12].try_into().unwrap()),
        })
    }
}

pub struct Wal {
    file: File,
    write_offset: usize,
}

impl Drop for Wal {
    fn drop(&mut self) {
        let _ = self.file.flush();
    }
}

impl Wal {
    pub fn create(path: &Path, page_size: u32) -> Result<Self> {
        let wal_path = Self::wal_path(path);

        let mut file = File::create(&wal_path, page_size)?;

        let mut header = WalHeader {
            magic: WAL_MAGIC,
            page_size,
            last_committed_pages: 0,
            checksum: 0,
        };
        header.checksum = header.compute_checksum();

        let header_bytes = header.to_bytes();
        let page = file.page_mut(0);
        page[..header_bytes.len()].copy_from_slice(&header_bytes);

        Ok(Self {
            file,
            write_offset: WalHeader::SIZE,
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let wal_path = Self::wal_path(path);

        if !wal_path.exists() {
            return Self::create(path, DEFAULT_PAGE_SIZE);
        }

        let file = File::open_raw(&wal_path, DEFAULT_PAGE_SIZE)?;

        let header_bytes = &file.page(0)[..WalHeader::SIZE];
        let header = WalHeader::from_bytes(header_bytes)?;

        if header.magic != WAL_MAGIC {
            return Err(Error::Wal("invalid WAL magic".into()));
        }

        let write_offset = Self::find_write_offset(&file);

        Ok(Self { file, write_offset })
    }

    pub fn append_page(&mut self, page_id: u32, data: &[u8]) -> Result<()> {
        let record_header = WalRecordHeader {
            page_id,
            data_length: data.len() as u32,
            checksum: crc32fast::hash(data),
        };

        let record_size = WalRecordHeader::SIZE + data.len();
        let required_size = self.write_offset + record_size;
        let current_size = self.file.total_pages() as usize * self.file.page_size() as usize;

        if required_size > current_size {
            self.file.grow()?;
        }

        let page_size = self.file.page_size() as usize;
        let offset_in_page = self.write_offset % page_size;
        let page_idx = self.write_offset / page_size;

        let page = self.file.page_mut(page_idx as u32);
        page[offset_in_page..offset_in_page + WalRecordHeader::SIZE]
            .copy_from_slice(&record_header.to_bytes());

        let data_start = offset_in_page + WalRecordHeader::SIZE;
        let data_end = data_start + data.len();
        page[data_start..data_end].copy_from_slice(data);

        self.write_offset += record_size;

        Ok(())
    }

    pub fn replay(&self, db_file: &mut File) -> Result<usize> {
        let header = WalHeader::from_bytes(&self.file.page(0)[..WalHeader::SIZE])?;

        if header.magic != WAL_MAGIC {
            return Err(Error::Wal("invalid WAL magic during replay".into()));
        }

        let mut offset = WalHeader::SIZE;
        let mut pages_replayed = 0;
        let page_size = self.file.page_size() as usize;

        while offset + WalRecordHeader::SIZE <= self.write_offset {
            let offset_in_page = offset % page_size;
            let page_idx = offset / page_size;
            let page = self.file.page(page_idx as u32);

            let record_header = match WalRecordHeader::from_bytes(&page[offset_in_page..]) {
                Ok(h) => h,
                Err(_) => break,
            };

            if record_header.data_length == 0 {
                break;
            }

            let data_start_in_page = offset_in_page + WalRecordHeader::SIZE;
            let data_len = record_header.data_length as usize;

            if data_start_in_page + data_len > page_size {
                break;
            }

            let data = &page[data_start_in_page..data_start_in_page + data_len];
            let computed_checksum = crc32fast::hash(data);

            if computed_checksum != record_header.checksum {
                let page_id = record_header.page_id;
                return Err(Error::Wal(format!(
                    "WAL record checksum mismatch for page {}",
                    page_id
                )));
            }

            let page_id = record_header.page_id;
            let required_pages = page_id + 1;
            while db_file.total_pages() < required_pages {
                db_file.grow()?;
            }

            let target_page = db_file.page_mut(page_id);
            let copy_len = data.len().min(target_page.len());
            target_page[..copy_len].copy_from_slice(&data[..copy_len]);

            pages_replayed += 1;
            offset += WalRecordHeader::SIZE + data_len;
        }

        Ok(pages_replayed)
    }

    pub fn checkpoint(&mut self, total_pages: u32) -> Result<()> {
        let page = self.file.page_mut(0);
        let mut header = WalHeader::from_bytes(&page[..WalHeader::SIZE])?;
        header.last_committed_pages = total_pages;
        header.checksum = header.compute_checksum();

        page[..WalHeader::SIZE].copy_from_slice(&header.to_bytes());

        self.write_offset = WalHeader::SIZE;

        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        let page_size = self.file.page_size();
        let page = self.file.page_mut(0);
        page.fill(0);

        let header = WalHeader {
            magic: WAL_MAGIC,
            page_size,
            last_committed_pages: 0,
            checksum: 0,
        };
        let header_bytes = header.to_bytes();
        page[..header_bytes.len()].copy_from_slice(&header_bytes);

        self.write_offset = WalHeader::SIZE;

        Ok(())
    }

    fn find_write_offset(file: &File) -> usize {
        let page_size = file.page_size() as usize;
        let mut offset = WalHeader::SIZE;

        loop {
            if offset + WalRecordHeader::SIZE > page_size {
                break;
            }

            let page = file.page(0);
            let record_header = match WalRecordHeader::from_bytes(&page[offset..]) {
                Ok(h) => h,
                Err(_) => break,
            };

            if record_header.data_length == 0 {
                break;
            }

            let data_len = record_header.data_length as usize;
            if offset + WalRecordHeader::SIZE + data_len > page_size {
                break;
            }

            offset += WalRecordHeader::SIZE + data_len;
        }

        offset
    }

    fn wal_path(db_path: &Path) -> PathBuf {
        let mut path = db_path.as_os_str().to_os_string();
        path.push("-wal");
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wal_create() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let wal = Wal::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        assert_eq!(wal.file.page_size(), DEFAULT_PAGE_SIZE);
    }

    #[test]
    fn test_wal_append_and_replay() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut wal = Wal::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let page_data = vec![0xAB; 100];
        wal.append_page(1, &page_data).unwrap();

        let mut db_file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        let pages_replayed = wal.replay(&mut db_file).unwrap();

        assert_eq!(pages_replayed, 1);
        assert_eq!(&db_file.page(1)[..100], &page_data[..]);
    }

    #[test]
    fn test_wal_checkpoint() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut wal = Wal::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        wal.checkpoint(10).unwrap();

        let page = wal.file.page(0);
        let header = WalHeader::from_bytes(&page[..WalHeader::SIZE]).unwrap();
        let last_committed_pages = header.last_committed_pages;
        assert_eq!(last_committed_pages, 10);
    }

    #[test]
    fn test_wal_clear() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        let mut wal = Wal::create(&path, DEFAULT_PAGE_SIZE).unwrap();
        wal.append_page(1, &[0xAB; 100]).unwrap();
        wal.clear().unwrap();

        let pages_replayed = wal
            .replay(&mut File::create(&path, DEFAULT_PAGE_SIZE).unwrap())
            .unwrap();
        assert_eq!(pages_replayed, 0);
    }

    #[test]
    fn test_wal_crash_recovery() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");

        {
            let mut wal = Wal::create(&path, DEFAULT_PAGE_SIZE).unwrap();
            wal.append_page(1, &[0xCD; 50]).unwrap();
            wal.append_page(2, &[0xEF; 50]).unwrap();
        }

        let wal = Wal::open(&path).unwrap();
        let mut db_file = File::create(&path, DEFAULT_PAGE_SIZE).unwrap();

        let pages_replayed = wal.replay(&mut db_file).unwrap();
        assert_eq!(pages_replayed, 2);
        assert_eq!(&db_file.page(1)[..50], &[0xCD; 50][..]);
        assert_eq!(&db_file.page(2)[..50], &[0xEF; 50][..]);
    }

    #[test]
    fn test_wal_header_roundtrip() {
        let header = WalHeader {
            magic: WAL_MAGIC,
            page_size: 4096,
            last_committed_pages: 42,
            checksum: 0,
        };

        let bytes = header.to_bytes();
        let parsed = WalHeader::from_bytes(&bytes).unwrap();

        let parsed_magic = parsed.magic;
        let header_magic = header.magic;
        assert_eq!(parsed_magic, header_magic);
        let parsed_page_size = parsed.page_size;
        let header_page_size = header.page_size;
        assert_eq!(parsed_page_size, header_page_size);
        let parsed_last_committed_pages = parsed.last_committed_pages;
        let header_last_committed_pages = header.last_committed_pages;
        assert_eq!(parsed_last_committed_pages, header_last_committed_pages);
    }

    #[test]
    fn test_wal_record_header_roundtrip() {
        let header = WalRecordHeader {
            page_id: 5,
            data_length: 100,
            checksum: 0xDEADBEEF,
        };

        let bytes = header.to_bytes();
        let parsed = WalRecordHeader::from_bytes(&bytes).unwrap();

        let parsed_page_id = parsed.page_id;
        let header_page_id = header.page_id;
        assert_eq!(parsed_page_id, header_page_id);
        let parsed_data_length = parsed.data_length;
        let header_data_length = header.data_length;
        assert_eq!(parsed_data_length, header_data_length);
        let parsed_checksum = parsed.checksum;
        let header_checksum = header.checksum;
        assert_eq!(parsed_checksum, header_checksum);
    }
}
