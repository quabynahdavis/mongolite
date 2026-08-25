use crate::error::Result;
use crate::storage::allocator::Allocator;
use crate::storage::file::{File, DEFAULT_PAGE_SIZE};
use crate::storage::page::{PageHeader, PageType};

pub type Key = Vec<u8>;
pub type Value = Vec<u8>;

#[derive(Debug, Clone)]
pub struct BTreeConfig {
    pub order: usize,
}

impl Default for BTreeConfig {
    fn default() -> Self {
        Self {
            order: max_order(DEFAULT_PAGE_SIZE),
        }
    }
}

pub(crate) fn max_order(page_size: u32) -> usize {
    let usable = page_size as usize - PageHeader::SIZE - 9;
    let entry_cost = 128;
    (usable / entry_cost).max(4)
}

const END_OF_CHAIN: u32 = 0xFFFFFFFF;

#[derive(Debug, Clone, Default)]
pub struct BTreeStats {
    pub height: usize,
    pub leaf_nodes: usize,
    pub internal_nodes: usize,
    pub total_keys: usize,
}

pub struct BTree<'a> {
    allocator: &'a mut Allocator<'a>,
    root_page: u32,
    config: BTreeConfig,
}

impl<'a> BTree<'a> {
    pub fn new(allocator: &'a mut Allocator<'a>, config: BTreeConfig) -> Result<Self> {
        let root_page = allocator.allocate()?;
        init_leaf_page(allocator.file_mut(), root_page);
        Ok(Self {
            allocator,
            root_page,
            config,
        })
    }

    pub fn open(
        allocator: &'a mut Allocator<'a>,
        root_page: u32,
        config: BTreeConfig,
    ) -> Result<Self> {
        Ok(Self {
            allocator,
            root_page,
            config,
        })
    }

    pub fn root_page(&self) -> u32 {
        self.root_page
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let result = self.insert_into_subtree(self.root_page, key, value)?;
        if let Some((median_key, right_page)) = result {
            let new_root = self.allocator.allocate()?;
            init_internal_page(self.allocator.file_mut(), new_root);
            let file = self.allocator.file_mut();
            let page_data = file.page_mut(new_root);
            write_u32_at(page_data, PageHeader::SIZE + 1, 1);
            let mut cursor = PageHeader::SIZE + 5;
            write_u32_at(page_data, cursor, self.root_page);
            cursor += 4;
            write_bytes_at(page_data, cursor, &median_key);
            cursor += 4 + median_key.len();
            write_u32_at(page_data, cursor, right_page);
            update_page_checksum(page_data);
            self.root_page = new_root;
        }
        Ok(())
    }

    fn insert_into_subtree(
        &mut self,
        page_id: u32,
        key: &[u8],
        value: &[u8],
    ) -> Result<Option<(Vec<u8>, u32)>> {
        if is_leaf_node(self.allocator.file(), page_id) {
            self.insert_into_leaf(page_id, key, value)
        } else {
            self.insert_into_internal(page_id, key, value)
        }
    }

    fn insert_into_leaf(
        &mut self,
        page_id: u32,
        key: &[u8],
        value: &[u8],
    ) -> Result<Option<(Vec<u8>, u32)>> {
        let num_keys = {
            let page_data = self.allocator.file().page(page_id);
            read_u32_at(page_data, PageHeader::SIZE + 1) as usize
        };

        if num_keys >= self.config.order {
            let split_point = num_keys / 2;
            let new_page = self.split_leaf_page(page_id, split_point)?;
            let new_leaf_first_key = {
                let page_data = self.allocator.file().page(new_page);
                read_bytes_at(page_data, PageHeader::SIZE + 9).to_vec()
            };

            if key < &new_leaf_first_key[..] {
                self.write_leaf_entry(page_id, key, value)?;
            } else {
                self.write_leaf_entry(new_page, key, value)?;
            }

            Ok(Some((new_leaf_first_key, new_page)))
        } else {
            self.write_leaf_entry(page_id, key, value)?;
            Ok(None)
        }
    }

    fn write_leaf_entry(
        &mut self,
        page_id: u32,
        key: &[u8],
        value: &[u8],
    ) -> Result<()> {
        let (num_keys, next_leaf, insert_pos) = {
            let page_data = self.allocator.file().page(page_id);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let next_leaf = read_u32_at(page_data, PageHeader::SIZE + 5);
            let insert_pos = find_insert_position(page_data, key, num_keys);
            (num_keys, next_leaf, insert_pos)
        };

        let file = self.allocator.file_mut();
        let page_data = file.page_mut(page_id);
        let data_start = PageHeader::SIZE + 9;

        let entry_offset = {
            let mut offset = data_start;
            for _ in 0..insert_pos {
                offset += entry_size_at(page_data, offset);
            }
            offset
        };

        let existing_data_len = {
            let mut len = 0;
            for i in 0..num_keys {
                let offset = data_start + len;
                len += entry_size_at(page_data, offset);
            }
            len
        };

        let new_entry_len = 4 + key.len() + 4 + value.len();
        let tail_offset = entry_offset;
        let tail_len = existing_data_len - (tail_offset - data_start);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
        buf.extend_from_slice(key);
        buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
        buf.extend_from_slice(value);
        if tail_len > 0 {
            buf.extend_from_slice(&page_data[tail_offset..tail_offset + tail_len]);
        }

        page_data[entry_offset..entry_offset + buf.len()].copy_from_slice(&buf);

        let new_num_keys = (num_keys + 1) as u32;
        write_u32_at(page_data, PageHeader::SIZE + 1, new_num_keys);
        write_u32_at(page_data, PageHeader::SIZE + 5, next_leaf);
        update_page_checksum(page_data);

        Ok(())
    }

    fn split_leaf_page(
        &mut self,
        page_id: u32,
        split_point: usize,
    ) -> Result<u32> {
        let (mut entries, next_leaf) = {
            let page_data = self.allocator.file().page(page_id);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let next_leaf = read_u32_at(page_data, PageHeader::SIZE + 5);
            let entries = read_all_leaf_entries(page_data, num_keys);
            (entries, next_leaf)
        };

        let right_entries = entries.split_off(split_point);
        let new_page = self.allocator.allocate()?;
        init_leaf_page(self.allocator.file_mut(), new_page);

        let file = self.allocator.file_mut();

        let new_page_data = file.page_mut(new_page);
        let mut cursor = PageHeader::SIZE + 9;
        for (k, v) in &right_entries {
            write_bytes_at(new_page_data, cursor, k);
            cursor += 4 + k.len();
            write_bytes_at(new_page_data, cursor, v);
            cursor += 4 + v.len();
        }
        write_u32_at(new_page_data, PageHeader::SIZE + 1, right_entries.len() as u32);
        write_u32_at(new_page_data, PageHeader::SIZE + 5, next_leaf);
        update_page_checksum(new_page_data);

        let page_data = file.page_mut(page_id);
        let mut cursor = PageHeader::SIZE + 9;
        for (k, v) in &entries {
            write_bytes_at(page_data, cursor, k);
            cursor += 4 + k.len();
            write_bytes_at(page_data, cursor, v);
            cursor += 4 + v.len();
        }
        write_u32_at(page_data, PageHeader::SIZE + 1, entries.len() as u32);
        write_u32_at(page_data, PageHeader::SIZE + 5, new_page);
        update_page_checksum(page_data);

        Ok(new_page)
    }

    fn insert_into_internal(
        &mut self,
        page_id: u32,
        key: &[u8],
        value: &[u8],
    ) -> Result<Option<(Vec<u8>, u32)>> {
        let (num_keys, child_page) = {
            let page_data = self.allocator.file().page(page_id);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let child_page = find_child_for_key(page_data, key, num_keys);
            (num_keys, child_page)
        };

        let split_result = self.insert_into_subtree(child_page, key, value)?;

        if let Some((median_key, right_page)) = split_result {
            self.add_key_to_internal(page_id, &median_key, right_page)?;

            let new_num_keys = {
                let page_data = self.allocator.file().page(page_id);
                read_u32_at(page_data, PageHeader::SIZE + 1) as usize
            };

            if new_num_keys > self.config.order {
                let split_point = new_num_keys / 2;
                let (promoted_key, new_internal_page) =
                    self.split_internal_page(page_id, split_point)?;
                Ok(Some((promoted_key, new_internal_page)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn add_key_to_internal(
        &mut self,
        page_id: u32,
        key: &[u8],
        child: u32,
    ) -> Result<()> {
        let (num_keys, insert_idx) = {
            let page_data = self.allocator.file().page(page_id);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let mut insert_idx = 0;
            let mut cursor = PageHeader::SIZE + 5 + 4;
            for _ in 0..num_keys {
                let k = read_bytes_at(page_data, cursor);
                if key < &k[..] {
                    break;
                }
                insert_idx += 1;
                cursor += 4 + k.len() + 4;
            }
            (num_keys, insert_idx)
        };

        let file = self.allocator.file_mut();
        let page_data = file.page_mut(page_id);

        let mut keys = Vec::with_capacity(num_keys);
        let mut children = Vec::with_capacity(num_keys + 1);
        {
            let mut cursor = PageHeader::SIZE + 5;
            children.push(read_u32_at(page_data, cursor));
            cursor += 4;
            for _ in 0..num_keys {
                let k = read_bytes_at(page_data, cursor).to_vec();
                cursor += 4 + k.len();
                let c = read_u32_at(page_data, cursor);
                cursor += 4;
                keys.push(k);
                children.push(c);
            }
        }

        keys.insert(insert_idx, key.to_vec());
        children.insert(insert_idx + 1, child);

        let mut cursor = PageHeader::SIZE + 5;
        write_u32_at(page_data, PageHeader::SIZE + 1, keys.len() as u32);
        write_u32_at(page_data, cursor, children[0]);
        cursor += 4;
        for (i, k) in keys.iter().enumerate() {
            write_bytes_at(page_data, cursor, k);
            cursor += 4 + k.len();
            write_u32_at(page_data, cursor, children[i + 1]);
            cursor += 4;
        }
        update_page_checksum(page_data);

        Ok(())
    }

    fn split_internal_page(
        &mut self,
        page_id: u32,
        split_point: usize,
    ) -> Result<(Vec<u8>, u32)> {
        let (keys, children) = {
            let page_data = self.allocator.file().page(page_id);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            read_all_internal_entries(page_data, num_keys)
        };

        let promoted_key = keys[split_point].clone();
        let left_keys = keys[..split_point].to_vec();
        let right_keys = keys[split_point + 1..].to_vec();
        let left_children = children[..=split_point].to_vec();
        let right_children = children[split_point + 1..].to_vec();

        let new_page = self.allocator.allocate()?;
        init_internal_page(self.allocator.file_mut(), new_page);

        let file = self.allocator.file_mut();

        let new_page_data = file.page_mut(new_page);
        {
            let mut cursor = PageHeader::SIZE + 5;
            write_u32_at(new_page_data, PageHeader::SIZE + 1, right_keys.len() as u32);
            write_u32_at(new_page_data, cursor, right_children[0]);
            cursor += 4;
            for (i, k) in right_keys.iter().enumerate() {
                write_bytes_at(new_page_data, cursor, k);
                cursor += 4 + k.len();
                write_u32_at(new_page_data, cursor, right_children[i + 1]);
                cursor += 4;
            }
            update_page_checksum(new_page_data);
        }

        let page_data = file.page_mut(page_id);
        {
            let mut cursor = PageHeader::SIZE + 5;
            write_u32_at(page_data, PageHeader::SIZE + 1, left_keys.len() as u32);
            write_u32_at(page_data, cursor, left_children[0]);
            cursor += 4;
            for (i, k) in left_keys.iter().enumerate() {
                write_bytes_at(page_data, cursor, k);
                cursor += 4 + k.len();
                write_u32_at(page_data, cursor, left_children[i + 1]);
                cursor += 4;
            }
            update_page_checksum(page_data);
        }

        Ok((promoted_key, new_page))
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Value>> {
        let mut page_id = self.root_page;
        loop {
            let page_data = self.allocator.file().page(page_id);
            if page_data[PageHeader::SIZE] == 1 {
                return Ok(leaf_search(page_data, key));
            }
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            page_id = find_child_for_key(page_data, key, num_keys);
        }
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<bool> {
        let (found, _) = self.delete_from_leaf(self.root_page, key)?;
        Ok(found)
    }

    fn delete_from_leaf(&mut self, page_id: u32, key: &[u8]) -> Result<(bool, usize)> {
        let page_data = self.allocator.file().page(page_id);
        if page_data[PageHeader::SIZE] == 1 {
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let mut cursor = PageHeader::SIZE + 9;
            for i in 0..num_keys {
                let k = read_bytes_at(page_data, cursor);
                let entry_start = cursor;
                cursor += 4 + k.len();
                let v_len = read_u32_at(page_data, cursor) as usize;
                cursor += 4 + v_len;
                if k == key {
                    let entry_end = cursor;
                    let file = self.allocator.file_mut();
                    let page_data = file.page_mut(page_id);
                    let tail_len = {
                        let mut end = PageHeader::SIZE + 9;
                        for _ in 0..num_keys {
                            end += entry_size_at(page_data, end);
                        }
                        end - entry_end
                    };
                    if tail_len > 0 {
                        page_data.copy_within(entry_end..entry_end + tail_len, entry_start);
                    }
                    write_u32_at(page_data, PageHeader::SIZE + 1, (num_keys - 1) as u32);
                    update_page_checksum(page_data);
                    return Ok((true, num_keys - 1));
                }
            }
            Ok((false, num_keys))
        } else {
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let child_page = find_child_for_key(page_data, key, num_keys);
            self.delete_from_leaf(child_page, key)
        }
    }

    pub fn range(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Key, Value)>> {
        let mut results = Vec::new();
        let mut current_page = self.find_leaf_page(start)?;
        loop {
            if current_page == END_OF_CHAIN {
                break;
            }
            let page_data = self.allocator.file().page(current_page);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let next_leaf = read_u32_at(page_data, PageHeader::SIZE + 5);
            let entries = read_all_leaf_entries(page_data, num_keys);
            for (k, v) in entries {
                if k.as_slice() >= start && k.as_slice() < end {
                    results.push((k, v));
                } else if k.as_slice() >= end {
                    return Ok(results);
                }
            }
            current_page = next_leaf;
        }
        Ok(results)
    }

    pub fn iter(&self) -> Result<Vec<(Key, Value)>> {
        let mut results = Vec::new();
        let mut current_page = self.leftmost_leaf(self.root_page)?;
        loop {
            if current_page == END_OF_CHAIN {
                break;
            }
            let page_data = self.allocator.file().page(current_page);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let next_leaf = read_u32_at(page_data, PageHeader::SIZE + 5);
            let entries = read_all_leaf_entries(page_data, num_keys);
            results.extend(entries);
            current_page = next_leaf;
        }
        Ok(results)
    }

    pub fn stats(&self) -> Result<BTreeStats> {
        let mut stats = BTreeStats {
            height: 0,
            leaf_nodes: 0,
            internal_nodes: 0,
            total_keys: 0,
        };
        self.collect_stats(self.root_page, 1, &mut stats)?;
        Ok(stats)
    }

    fn collect_stats(&self, page_id: u32, depth: usize, stats: &mut BTreeStats) -> Result<()> {
        if depth > stats.height {
            stats.height = depth;
        }
        let page_data = self.allocator.file().page(page_id);
        let is_leaf = page_data[PageHeader::SIZE] == 1;
        let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
        if is_leaf {
            stats.leaf_nodes += 1;
            stats.total_keys += num_keys;
        } else {
            stats.internal_nodes += 1;
            let mut cursor = PageHeader::SIZE + 5;
            for i in 0..=num_keys {
                let child = read_u32_at(page_data, cursor);
                cursor += 4;
                if i < num_keys {
                    let k_len = read_u32_at(page_data, cursor) as usize;
                    cursor += 4 + k_len;
                }
                self.collect_stats(child, depth + 1, stats)?;
            }
        }
        Ok(())
    }

    pub fn is_empty(&self) -> Result<bool> {
        let page_data = self.allocator.file().page(self.root_page);
        if page_data[PageHeader::SIZE] == 1 {
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1);
            Ok(num_keys == 0)
        } else {
            Ok(false)
        }
    }

    pub fn len(&self) -> Result<usize> {
        let mut total = 0;
        let mut current_page = self.leftmost_leaf(self.root_page)?;
        loop {
            if current_page == END_OF_CHAIN {
                break;
            }
            let page_data = self.allocator.file().page(current_page);
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            let next_leaf = read_u32_at(page_data, PageHeader::SIZE + 5);
            total += num_keys;
            current_page = next_leaf;
        }
        Ok(total)
    }

    fn find_leaf_page(&self, key: &[u8]) -> Result<u32> {
        let mut page_id = self.root_page;
        loop {
            let page_data = self.allocator.file().page(page_id);
            if page_data[PageHeader::SIZE] == 1 {
                return Ok(page_id);
            }
            let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
            page_id = find_child_for_key(page_data, key, num_keys);
        }
    }

    fn leftmost_leaf(&self, mut page_id: u32) -> Result<u32> {
        loop {
            let page_data = self.allocator.file().page(page_id);
            if page_data[PageHeader::SIZE] == 1 {
                return Ok(page_id);
            }
            page_id = read_u32_at(page_data, PageHeader::SIZE + 5);
        }
    }
}

fn is_leaf_node(file: &File, page_id: u32) -> bool {
    file.page(page_id)[PageHeader::SIZE] == 1
}

fn init_leaf_page(file: &mut File, page_id: u32) {
    let page_data = file.page_mut(page_id);
    page_data[..PageHeader::SIZE].copy_from_slice(
        &PageHeader {
            page_type: PageType::BTreeLeaf as u8,
            flags: 0,
            padding: 0,
            checksum: 0,
        }
        .to_bytes(),
    );
    page_data[PageHeader::SIZE] = 1;
    page_data[PageHeader::SIZE + 1..PageHeader::SIZE + 5].copy_from_slice(&0u32.to_le_bytes());
    page_data[PageHeader::SIZE + 5..PageHeader::SIZE + 9].copy_from_slice(&END_OF_CHAIN.to_le_bytes());
    page_data[PageHeader::SIZE + 9..].fill(0);
    update_page_checksum(page_data);
}

fn init_internal_page(file: &mut File, page_id: u32) {
    let page_data = file.page_mut(page_id);
    page_data[..PageHeader::SIZE].copy_from_slice(
        &PageHeader {
            page_type: PageType::BTreeInternal as u8,
            flags: 0,
            padding: 0,
            checksum: 0,
        }
        .to_bytes(),
    );
    page_data[PageHeader::SIZE] = 0;
    page_data[PageHeader::SIZE + 1..PageHeader::SIZE + 5].copy_from_slice(&0u32.to_le_bytes());
    page_data[PageHeader::SIZE + 5..].fill(0);
    update_page_checksum(page_data);
}

fn leaf_search(page_data: &[u8], key: &[u8]) -> Option<Value> {
    let num_keys = read_u32_at(page_data, PageHeader::SIZE + 1) as usize;
    let entries = read_all_leaf_entries(page_data, num_keys);
    for (k, v) in entries {
        if k.as_slice() == key {
            return Some(v);
        }
    }
    None
}

fn find_child_for_key(page_data: &[u8], key: &[u8], num_keys: usize) -> u32 {
    let mut cursor = PageHeader::SIZE + 5;
    let mut last_child = read_u32_at(page_data, cursor);
    cursor += 4;
    for _ in 0..num_keys {
        let k = read_bytes_at(page_data, cursor);
        cursor += 4 + k.len();
        let child = read_u32_at(page_data, cursor);
        cursor += 4;
        if key < &k[..] {
            return last_child;
        }
        last_child = child;
    }
    last_child
}

fn find_insert_position(page_data: &[u8], key: &[u8], num_keys: usize) -> usize {
    let mut cursor = PageHeader::SIZE + 9;
    for pos in 0..num_keys {
        let k = read_bytes_at(page_data, cursor);
        if key <= &k[..] {
            return pos;
        }
        cursor += 4 + k.len() + 4;
        let v_len = read_u32_at(page_data, cursor - 4) as usize;
        cursor += v_len;
    }
    num_keys
}

fn entry_size_at(page_data: &[u8], offset: usize) -> usize {
    let k_len = read_u32_at(page_data, offset) as usize;
    let v_len = read_u32_at(page_data, offset + 4 + k_len) as usize;
    4 + k_len + 4 + v_len
}

fn read_all_leaf_entries(page_data: &[u8], num_keys: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut entries = Vec::with_capacity(num_keys);
    let mut cursor = PageHeader::SIZE + 9;
    for _ in 0..num_keys {
        let k = read_bytes_at(page_data, cursor).to_vec();
        cursor += 4 + k.len();
        let v_len = read_u32_at(page_data, cursor) as usize;
        cursor += 4;
        let v = page_data[cursor..cursor + v_len].to_vec();
        cursor += v_len;
        entries.push((k, v));
    }
    entries
}

fn read_all_internal_entries(page_data: &[u8], num_keys: usize) -> (Vec<Vec<u8>>, Vec<u32>) {
    let mut keys = Vec::with_capacity(num_keys);
    let mut children = Vec::with_capacity(num_keys + 1);
    let mut cursor = PageHeader::SIZE + 5;
    children.push(read_u32_at(page_data, cursor));
    cursor += 4;
    for _ in 0..num_keys {
        let k = read_bytes_at(page_data, cursor).to_vec();
        cursor += 4 + k.len();
        let c = read_u32_at(page_data, cursor);
        cursor += 4;
        keys.push(k);
        children.push(c);
    }
    (keys, children)
}

fn read_u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn write_u32_at(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn read_bytes_at(data: &[u8], offset: usize) -> &[u8] {
    let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    &data[offset + 4..offset + 4 + len]
}

fn write_bytes_at(data: &mut [u8], offset: usize, bytes: &[u8]) {
    data[offset..offset + 4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    data[offset + 4..offset + 4 + bytes.len()].copy_from_slice(bytes);
}

fn update_page_checksum(page_data: &mut [u8]) {
    let checksum = crc32fast::hash(&page_data[PageHeader::SIZE..]);
    page_data[4..8].copy_from_slice(&checksum.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::file::File;
    use tempfile::TempDir;

    fn create_test_file() -> (File, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        (File::create(&path, DEFAULT_PAGE_SIZE).unwrap(), dir)
    }

    fn create_test_tree(order: usize) -> BTree<'static> {
        let (file, dir) = create_test_file();
        let file = Box::leak(Box::new(file));
        let allocator = Box::leak(Box::new(Allocator::new(file)));
        let config = BTreeConfig { order };
        std::mem::forget(dir);
        BTree::new(allocator, config).unwrap()
    }

    #[test]
    fn test_empty_tree() {
        let tree = create_test_tree(4);
        assert!(tree.is_empty().unwrap());
        assert_eq!(tree.len().unwrap(), 0);
        assert!(tree.get(b"any_key").unwrap().is_none());
    }

    #[test]
    fn test_insert_and_get_single() {
        let mut tree = create_test_tree(4);
        tree.insert(b"hello", b"world").unwrap();
        assert!(!tree.is_empty().unwrap());
        assert_eq!(tree.len().unwrap(), 1);
        assert_eq!(tree.get(b"hello").unwrap(), Some(b"world".to_vec()));
        assert!(tree.get(b"missing").unwrap().is_none());
    }

    #[test]
    fn test_insert_and_get_multiple() {
        let mut tree = create_test_tree(128);
        for k in &["a", "b", "c", "d", "e"] {
            tree.insert(k.as_bytes(), b"val").unwrap();
        }
        assert_eq!(tree.len().unwrap(), 5);
        assert_eq!(tree.get(b"a").unwrap(), Some(b"val".to_vec()));
        assert_eq!(tree.get(b"c").unwrap(), Some(b"val".to_vec()));
        assert!(tree.get(b"f").unwrap().is_none());
    }

    #[test]
    fn test_insert_causes_split() {
        let mut tree = create_test_tree(3);
        for i in 0..10u32 {
            tree.insert(format!("key{:02}", i).as_bytes(), format!("val{:02}", i).as_bytes()).unwrap();
        }
        assert_eq!(tree.len().unwrap(), 10);
        for i in 0..10u32 {
            assert_eq!(
                tree.get(format!("key{:02}", i).as_bytes()).unwrap(),
                Some(format!("val{:02}", i).into_bytes())
            );
        }
    }

    #[test]
    fn test_delete() {
        let mut tree = create_test_tree(128);
        tree.insert(b"a", b"1").unwrap();
        tree.insert(b"b", b"2").unwrap();
        tree.insert(b"c", b"3").unwrap();
        assert!(tree.delete(b"b").unwrap());
        assert_eq!(tree.len().unwrap(), 2);
        assert!(tree.get(b"b").unwrap().is_none());
        assert!(!tree.delete(b"nonexistent").unwrap());
    }

    #[test]
    fn test_delete_causes_no_rebalance() {
        let mut tree = create_test_tree(3);
        for i in 0..10u32 {
            tree.insert(format!("key{:02}", i).as_bytes(), format!("val{:02}", i).as_bytes()).unwrap();
        }
        for i in 0..5u32 {
            assert!(tree.delete(format!("key{:02}", i).as_bytes()).unwrap());
        }
        assert_eq!(tree.len().unwrap(), 5);
        for i in 5..10u32 {
            assert_eq!(
                tree.get(format!("key{:02}", i).as_bytes()).unwrap(),
                Some(format!("val{:02}", i).into_bytes())
            );
        }
    }

    #[test]
    fn test_range_scan() {
        let mut tree = create_test_tree(128);
        for i in 0..26u32 {
            let key = vec![b'a' + i as u8];
            tree.insert(&key, &format!("val{}", i).into_bytes()).unwrap();
        }
        let results = tree.range(b"c", b"g").unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].0, b"c");
    }

    #[test]
    fn test_iter() {
        let mut tree = create_test_tree(128);
        for k in &["c", "a", "b", "e", "d"] {
            tree.insert(k.as_bytes(), b"val").unwrap();
        }
        let results = tree.iter().unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].0, b"a");
        assert_eq!(results[1].0, b"b");
        assert_eq!(results[2].0, b"c");
        assert_eq!(results[3].0, b"d");
        assert_eq!(results[4].0, b"e");
    }

    #[test]
    fn test_stats() {
        let mut tree = create_test_tree(3);
        for i in 0..20u32 {
            tree.insert(format!("key{:03}", i).as_bytes(), b"val").unwrap();
        }
        let stats = tree.stats().unwrap();
        assert!(stats.height >= 2);
        assert!(stats.leaf_nodes >= 2);
        assert!(stats.internal_nodes >= 1);
        assert_eq!(stats.total_keys, 20);
    }

    #[test]
    fn test_medium_insert() {
        let mut tree = create_test_tree(4);
        for i in 0..10u32 {
            tree.insert(format!("k{:02}", i).as_bytes(), format!("v{:02}", i).as_bytes()).unwrap();
        }
        assert_eq!(tree.len().unwrap(), 10);
        for i in 0..10u32 {
            assert_eq!(
                tree.get(format!("k{:02}", i).as_bytes()).unwrap(),
                Some(format!("v{:02}", i).into_bytes())
            );
        }
    }

    #[test]
    fn test_keys_sorted_order() {
        let mut tree = create_test_tree(4);
        for k in &[5u32, 3, 8, 1, 9, 2, 7, 4, 6, 0] {
            tree.insert(format!("{}", k).as_bytes(), b"val").unwrap();
        }
        let results = tree.iter().unwrap();
        for (i, (k, _)) in results.iter().enumerate() {
            assert_eq!(k, &format!("{}", i).into_bytes());
        }
    }

    #[test]
    fn test_overwrite_existing_key() {
        let mut tree = create_test_tree(128);
        tree.insert(b"key", b"original").unwrap();
        tree.insert(b"key", b"updated").unwrap();
        assert_eq!(tree.get(b"key").unwrap(), Some(b"updated".to_vec()));
    }

    #[test]
    fn test_range_empty_result() {
        let mut tree = create_test_tree(128);
        tree.insert(b"a", b"1").unwrap();
        tree.insert(b"b", b"2").unwrap();
        assert!(tree.range(b"c", b"z").unwrap().is_empty());
    }

    #[test]
    fn test_range_full_range() {
        let mut tree = create_test_tree(128);
        for i in 0..10u32 {
            let key = vec![b'a' + i as u8];
            tree.insert(&key, b"val").unwrap();
        }
        assert_eq!(tree.range(b"a", b"z").unwrap().len(), 10);
    }

    #[test]
    fn test_open_existing_tree() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mongolite");
        let root_page = {
            let file = Box::leak(Box::new(File::create(&path, DEFAULT_PAGE_SIZE).unwrap()));
            let allocator = Box::leak(Box::new(Allocator::new(file)));
            let mut tree = BTree::new(allocator, BTreeConfig { order: 4 }).unwrap();
            tree.insert(b"persist_key", b"persist_value").unwrap();
            tree.root_page()
        };
        let file = Box::leak(Box::new(File::open(&path).unwrap()));
        let allocator = Box::leak(Box::new(Allocator::new(file)));
        let tree = BTree::open(allocator, root_page, BTreeConfig { order: 4 }).unwrap();
        assert_eq!(tree.get(b"persist_key").unwrap(), Some(b"persist_value".to_vec()));
        std::mem::forget(dir);
    }
}
