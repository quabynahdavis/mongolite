use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    Free = 0x00,
    BTreeLeaf = 0x01,
    BTreeInternal = 0x02,
    Overflow = 0x03,
    Catalog = 0x04,
}

impl PageType {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Free),
            0x01 => Ok(Self::BTreeLeaf),
            0x02 => Ok(Self::BTreeInternal),
            0x03 => Ok(Self::Overflow),
            0x04 => Ok(Self::Catalog),
            _ => Err(Error::Corrupted(format!(
                "invalid page type: 0x{:02x}",
                value
            ))),
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PageHeader {
    pub page_type: u8,
    pub flags: u8,
    pub padding: u16,
    pub checksum: u32,
}

impl PageHeader {
    pub const SIZE: usize = 1 + 1 + 2 + 4;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = self.page_type;
        buf[1] = self.flags;
        buf[2..4].copy_from_slice(&self.padding.to_le_bytes());
        buf[4..8].copy_from_slice(&self.checksum.to_le_bytes());
        buf
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(Error::Corrupted("page header too short".into()));
        }

        Ok(Self {
            page_type: data[0],
            flags: data[1],
            padding: u16::from_le_bytes(data[2..4].try_into().unwrap()),
            checksum: u32::from_le_bytes(data[4..8].try_into().unwrap()),
        })
    }
}

pub struct Page<'a> {
    data: &'a [u8],
    header: PageHeader,
}

impl<'a> Page<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let header = PageHeader::from_bytes(data)?;
        Ok(Self { data, header })
    }

    pub fn page_type(&self) -> Result<PageType> {
        PageType::from_u8(self.header.page_type)
    }

    pub fn validate_checksum(&self) -> bool {
        let data_start = PageHeader::SIZE;
        if self.data.len() <= data_start {
            return true;
        }
        let page_data = &self.data[data_start..];
        let computed = crc32fast::hash(page_data);
        computed == self.header.checksum
    }

    pub fn data(&self) -> &[u8] {
        &self.data[PageHeader::SIZE..]
    }
}

pub struct PageMut<'a> {
    data: &'a mut [u8],
    header: PageHeader,
}

impl<'a> PageMut<'a> {
    pub fn new(data: &'a mut [u8]) -> Result<Self> {
        let header = PageHeader::from_bytes(data)?;
        Ok(Self { data, header })
    }

    pub fn set_type(&mut self, page_type: PageType) {
        self.header.page_type = page_type as u8;
        self.data[0] = page_type as u8;
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.header.flags = flags;
        self.data[1] = flags;
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[PageHeader::SIZE..]
    }

    pub fn compute_checksum(&mut self) {
        let data_start = PageHeader::SIZE;
        let page_data = &self.data[data_start..];
        self.header.checksum = crc32fast::hash(page_data);
        self.data[4..8].copy_from_slice(&self.header.checksum.to_le_bytes());
    }

    pub fn write_header(&mut self) {
        let bytes = self.header.to_bytes();
        self.data[..PageHeader::SIZE].copy_from_slice(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_type_from_u8() {
        assert_eq!(PageType::from_u8(0x00).unwrap(), PageType::Free);
        assert_eq!(PageType::from_u8(0x01).unwrap(), PageType::BTreeLeaf);
        assert_eq!(PageType::from_u8(0x02).unwrap(), PageType::BTreeInternal);
        assert_eq!(PageType::from_u8(0x03).unwrap(), PageType::Overflow);
        assert_eq!(PageType::from_u8(0x04).unwrap(), PageType::Catalog);
        assert!(PageType::from_u8(0xFF).is_err());
    }

    #[test]
    fn test_page_header_roundtrip() {
        let header = PageHeader {
            page_type: 0x01,
            flags: 0x02,
            padding: 0x0304,
            checksum: 0x05060708,
        };

        let bytes = header.to_bytes();
        let parsed = PageHeader::from_bytes(&bytes).unwrap();

        let parsed_page_type = parsed.page_type;
        let header_page_type = header.page_type;
        assert_eq!(parsed_page_type, header_page_type);
        let parsed_flags = parsed.flags;
        let header_flags = header.flags;
        assert_eq!(parsed_flags, header_flags);
        let parsed_padding = parsed.padding;
        let header_padding = header.padding;
        assert_eq!(parsed_padding, header_padding);
        let parsed_checksum = parsed.checksum;
        let header_checksum = header.checksum;
        assert_eq!(parsed_checksum, header_checksum);
    }

    #[test]
    fn test_page_new() {
        let mut data = vec![0u8; 64];
        data[0] = 0x01; // BTreeLeaf
        data[1] = 0x00; // flags
        data[4..8].copy_from_slice(&0u32.to_le_bytes()); // checksum

        let page = Page::new(&data).unwrap();
        assert_eq!(page.page_type().unwrap(), PageType::BTreeLeaf);
        assert_eq!(page.data().len(), 64 - PageHeader::SIZE);
    }

    #[test]
    fn test_page_checksum() {
        let mut data = vec![0u8; 64];
        data[0] = 0x01;
        let page_data = &data[PageHeader::SIZE..];
        let checksum = crc32fast::hash(page_data);
        data[4..8].copy_from_slice(&checksum.to_le_bytes());

        let page = Page::new(&data).unwrap();
        assert!(page.validate_checksum());
    }

    #[test]
    fn test_page_checksum_invalid() {
        let mut data = vec![0u8; 64];
        data[0] = 0x01;
        data[4..8].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());

        let page = Page::new(&data).unwrap();
        assert!(!page.validate_checksum());
    }

    #[test]
    fn test_page_mut_set_type() {
        let mut data = vec![0u8; 64];
        let mut page = PageMut::new(&mut data).unwrap();
        page.set_type(PageType::Catalog);

        assert_eq!(data[0], 0x04);
    }

    #[test]
    fn test_page_mut_data() {
        let mut data = vec![0u8; 64];
        let mut page = PageMut::new(&mut data).unwrap();
        let page_data = page.data_mut();
        page_data[0] = 0x42;

        assert_eq!(data[PageHeader::SIZE], 0x42);
    }

    #[test]
    fn test_page_mut_compute_checksum() {
        let mut data = vec![0u8; 64];
        data[PageHeader::SIZE] = 0xAB;
        data[PageHeader::SIZE + 1] = 0xCD;

        let mut page = PageMut::new(&mut data).unwrap();
        page.compute_checksum();

        let expected = crc32fast::hash(&data[PageHeader::SIZE..]);
        let stored = u32::from_le_bytes(data[4..8].try_into().unwrap());
        assert_eq!(stored, expected);
    }

    #[test]
    fn test_page_mut_write_header() {
        let mut data = vec![0u8; 64];
        let mut page = PageMut::new(&mut data).unwrap();
        page.set_type(PageType::BTreeInternal);
        page.set_flags(0xFF);
        page.compute_checksum();
        page.write_header();

        let header = PageHeader::from_bytes(&data).unwrap();
        assert_eq!(header.page_type, 0x02);
        assert_eq!(header.flags, 0xFF);
    }
}
