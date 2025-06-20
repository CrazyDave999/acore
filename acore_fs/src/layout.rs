use crate::block_manager::get_block_cache;
use crate::{BlockDevice, BLOCK_SIZE};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cmp::min;

const AFS_MAGIC_NUM: u32 = 0x114514;

/// should be BLOCK_SIZE bytes
#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_blocks: u32,
    pub _other_msg: [u8; BLOCK_SIZE - 20],
}

impl SuperBlock {
    pub fn init(
        &mut self,
        inode_bitmap_blocks: u32,
        inode_blocks: u32,
        data_bitmap_blocks: u32,
        data_blocks: u32,
    ) {
        self.magic = AFS_MAGIC_NUM;
        self.inode_bitmap_blocks = inode_bitmap_blocks;
        self.inode_blocks = inode_blocks;
        self.data_bitmap_blocks = data_bitmap_blocks;
        self.data_blocks = data_blocks;
    }
    pub fn is_valid(&self) -> bool {
        self.magic == AFS_MAGIC_NUM
    }
}

const INODE_DIRECT_CNT: usize = 26;
const MAX_INDIRECT_DEGREE: usize = 3;
/// how much inodes are there in one indirect inode block
const INDIRECT_CNT: usize = BLOCK_SIZE / core::mem::size_of::<u32>();

type IndirectBlock = [u32; INDIRECT_CNT];

/// size should be 128 bytes
/// 26 + 128 + 128^2 + 128^3 data blocks in total
#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_CNT],
    pub indirect: [u32; MAX_INDIRECT_DEGREE],
    pub next: u32,
    pub type_: DiskInodeType,
}

pub const DISK_INODE_SIZE: usize = core::mem::size_of::<DiskInode>();
pub const DISK_INODE_PER_BLOCK: usize = BLOCK_SIZE / DISK_INODE_SIZE;

#[derive(Clone, Copy)]
pub enum DiskInodeType {
    File,
    Directory,
}

impl DiskInode {
    pub fn init(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct = [0; INODE_DIRECT_CNT];
        self.indirect = [0; MAX_INDIRECT_DEGREE];
        self.next = 0;
        self.type_ = type_;
    }
    pub fn is_dir(&self) -> bool {
        matches!(self.type_, DiskInodeType::Directory)
    }
    // pub fn is_file(&self) -> bool {
    //     matches!(self.type_, DiskInodeType::File)
    // }

    /// Get block_id with given inner_id
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let mut inner_id = inner_id as usize;

        if inner_id < INODE_DIRECT_CNT {
            assert!(
                self.direct[inner_id] < 1000_000,
                "inner_id: {}, self.direct[inner_id]: {}",
                inner_id,
                self.direct[inner_id]
            );
            return self.direct[inner_id];
        }

        let end_id = INODE_DIRECT_CNT + {
            let mut res = 0;
            for degree in 0..MAX_INDIRECT_DEGREE {
                res += INDIRECT_CNT.pow((degree + 1) as u32);
            }
            res
        };

        assert!(
            inner_id < end_id,
            "inner_id {} out of range 0-{}",
            inner_id,
            end_id
        );

        inner_id -= INODE_DIRECT_CNT;
        // cur total end id for this degree's indirect block can manage
        for (degree, &root_id) in self.indirect.iter().enumerate() {
            let full_num = INDIRECT_CNT.pow((degree + 1) as u32);
            if inner_id >= full_num {
                inner_id -= full_num;
                continue;
            }

            let mut block_id = root_id;

            // println!("root_id: {}, degree: {}, inner_id: {}", root_id, degree, inner_id);

            for depth in 0..=degree {
                // cur node
                let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
                let indirect_block_lock = cache.lock();
                let indirect_block = indirect_block_lock.as_ref::<IndirectBlock>(0);

                let son_full_num = INDIRECT_CNT.pow((degree - depth) as u32);
                block_id = indirect_block[inner_id / son_full_num];

                assert!(
                    block_id < 1000_000,
                    "inner_id: {}, block_id: {}",
                    inner_id,
                    block_id
                );

                inner_id %= son_full_num;
            }

            return block_id;
        }
        panic!("Should not reach here");
    }

    /// read from current inode(file)
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = min(offset + buf.len(), self.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_inner_block_id = start / BLOCK_SIZE;
        let mut read_size = 0usize;
        loop {
            let end_current_block = min(end, (start / BLOCK_SIZE + 1) * BLOCK_SIZE);
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            let block_id = self.get_block_id(start_inner_block_id as u32, block_device);

            let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
            let data_block_lock = cache.lock();
            let data_block = data_block_lock.as_ref::<DataBlock>(0);

            let src = &data_block.0[start % BLOCK_SIZE..start % BLOCK_SIZE + block_read_size];
            dst.copy_from_slice(src);
            read_size += block_read_size;

            if end_current_block == end {
                break;
            }
            start = end_current_block;
            start_inner_block_id += 1;
        }
        read_size
    }

    /// write to current inode(file)
    pub fn write_at(
        &self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        // println!(
        //     "write_at. offset: {}, buf.len: {}, self.size: {}",
        //     offset,
        //     buf.len(),
        //     self.size
        // );
        let mut start = offset;
        let end = min(offset + buf.len(), self.size as usize);
        assert!(end > start, "Invalid write. start: {}, end: {}", start, end);
        let mut start_inner_block_id = start / BLOCK_SIZE;
        let mut write_size = 0usize;
        loop {
            let end_current_block = min(end, (start / BLOCK_SIZE + 1) * BLOCK_SIZE);
            let block_write_size = end_current_block - start;
            let block_id = self.get_block_id(start_inner_block_id as u32, block_device);

            let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
            let mut data_block_lock = cache.lock();
            let data_block = data_block_lock.as_mut_ref::<DataBlock>(0);

            let dst = &mut data_block.0[start % BLOCK_SIZE..start % BLOCK_SIZE + block_write_size];
            let src = &buf[write_size..write_size + block_write_size];
            dst.copy_from_slice(src);
            write_size += block_write_size;
            if end_current_block == end {
                break;
            }
            start = end_current_block;
            start_inner_block_id += 1;
        }
        write_size
    }
    /// Return the number of data blocks used by this inode, not including the indirect blocks.
    fn data_blocks(&self) -> u32 {
        (self.size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    /// Return number of blocks needed including indirect blocks
    pub fn total_blocks(size: u32) -> u32 {
        let mut data_blocks = (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32;
        let mut total_blocks = data_blocks;

        // calculate indirect blocks
        if data_blocks <= INODE_DIRECT_CNT as u32 {
            return total_blocks;
        }

        data_blocks -= INODE_DIRECT_CNT as u32;

        for degree in 0..MAX_INDIRECT_DEGREE {
            total_blocks += 1; // root
            let full_num = INDIRECT_CNT.pow((degree + 1) as u32) as u32;
            let cur_degree_data_blocks = min(data_blocks, full_num);
            for depth in 0..degree {
                // non-leaf layer
                let son_full_num = INDIRECT_CNT.pow((degree - depth) as u32) as u32;
                total_blocks += (cur_degree_data_blocks + son_full_num - 1) / son_full_num;
            }
            data_blocks -= cur_degree_data_blocks;
            if data_blocks == 0 {
                break;
            }
        }

        total_blocks
    }

    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    /// Increase size of current inode, with given new data blocks
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut cur_data_blocks = self.data_blocks();
        self.size = new_size;
        let mut new_data_blocks = self.data_blocks();
        // println!(
        //     "increase_size. cur_data_blocks: {}, new_data_blocks: {}, new_blocks.len: {}",
        //     cur_data_blocks,
        //     new_data_blocks,
        //     new_blocks.len()
        // );
        let mut new_blocks_iter = new_blocks.into_iter();

        // fill in the direct blocks
        while cur_data_blocks < min(INODE_DIRECT_CNT as u32, new_data_blocks) {
            self.direct[cur_data_blocks as usize] = new_blocks_iter.next().unwrap();
            cur_data_blocks += 1;
        }

        if new_data_blocks <= INODE_DIRECT_CNT as u32 {
            return;
        }

        cur_data_blocks -= INODE_DIRECT_CNT as u32;
        new_data_blocks -= INODE_DIRECT_CNT as u32;

        // println!(
        //     "increase_size. cur_data_blocks: {}, new_data_blocks: {}, new_blocks_iter.len: {}",
        //     cur_data_blocks,
        //     new_data_blocks,
        //     new_blocks_iter.clone().count()
        // );

        for i in 0..MAX_INDIRECT_DEGREE {
            let full_num = INDIRECT_CNT.pow((i + 1) as u32) as u32;

            // println!(
            //     "increase_size. i: {}, full_num: {}, cur_data_blocks: {}, new_data_blocks: {}",
            //     i, full_num, cur_data_blocks, new_data_blocks
            // );

            if cur_data_blocks >= full_num {
                cur_data_blocks -= full_num;
                new_data_blocks -= full_num;
                continue;
            }

            let cur_sub_tree_data_blocks = cur_data_blocks;
            let new_sub_tree_data_blocks = min(full_num, new_data_blocks);

            // println!(
            //     "increase_size. i: {}, cur_sub_tree_data_blocks: {}, new_sub_tree_data_blocks: {}",
            //     i, cur_sub_tree_data_blocks, new_sub_tree_data_blocks
            // );

            if cur_sub_tree_data_blocks == 0 {
                self.indirect[i] = new_blocks_iter.next().unwrap();

                // println!(
                //     "increase_size. i: {}, self.indirect[i]: {}",
                //     i, self.indirect[i]
                // );
            }

            Self::dfs_fill_indirect(
                0,
                i,
                self.indirect[i],
                cur_sub_tree_data_blocks,
                new_sub_tree_data_blocks,
                &mut new_blocks_iter,
                block_device,
            );

            cur_data_blocks = 0;
            new_data_blocks -= new_sub_tree_data_blocks;

            if new_data_blocks == 0 {
                break;
            }
        }
        assert!(new_blocks_iter.next().is_none());
    }

    fn dfs_fill_indirect(
        depth: usize,
        degree: usize,
        block_id: u32,
        mut cur_sub_tree_data_blocks: u32,
        mut new_sub_tree_data_blocks: u32,
        new_blocks_iter: &mut dyn Iterator<Item = u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        // get cur nodes
        let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
        let mut indirect_block_lock = cache.lock();
        let indirect_block = indirect_block_lock.as_mut_ref::<IndirectBlock>(0);

        if depth == degree {
            // leaf node
            for i in cur_sub_tree_data_blocks as usize..new_sub_tree_data_blocks as usize {
                indirect_block[i] = new_blocks_iter.next().unwrap();
            }
            return;
        }

        // not leaf node
        // max data blocks that cur node's son can hold in its subtree
        let son_full_num = INDIRECT_CNT.pow((degree - depth) as u32) as u32;

        for i in 0..INDIRECT_CNT {
            if cur_sub_tree_data_blocks >= son_full_num {
                cur_sub_tree_data_blocks -= son_full_num;
                new_sub_tree_data_blocks -= son_full_num;
                continue;
            }

            let son_cur_sub_tree_data_blocks = cur_sub_tree_data_blocks;
            let son_new_sub_tree_data_blocks = min(son_full_num, new_sub_tree_data_blocks);

            if son_cur_sub_tree_data_blocks == 0 {
                indirect_block[i] = new_blocks_iter.next().unwrap();
            }
            Self::dfs_fill_indirect(
                depth + 1,
                degree,
                indirect_block[i],
                son_cur_sub_tree_data_blocks,
                son_new_sub_tree_data_blocks,
                new_blocks_iter,
                block_device,
            );

            cur_sub_tree_data_blocks = 0;
            new_sub_tree_data_blocks -= son_new_sub_tree_data_blocks;

            if new_sub_tree_data_blocks == 0 {
                break;
            }
        }
    }

    /// Clear size of current inode, return the block_ids of data blocks, including indirect blocks.
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let data_blocks = self.data_blocks() as usize;
        self.size = 0;

        // the data blocks cleared currently
        let mut cur_data_blocks = 0usize;

        while cur_data_blocks < min(INODE_DIRECT_CNT, data_blocks) {
            v.push(self.direct[cur_data_blocks]);
            self.direct[cur_data_blocks] = 0;
            cur_data_blocks += 1;
        }
        if data_blocks <= INODE_DIRECT_CNT {
            return v;
        }

        for i in 0..MAX_INDIRECT_DEGREE {
            if self.indirect[i] != 0 {
                v.push(self.indirect[i]);
                v.extend(Self::dfs_clear_indirect(
                    0,
                    i,
                    self.indirect[i],
                    block_device,
                ));
            }
            self.indirect[i] = 0;
        }
        v
    }

    fn dfs_clear_indirect(
        depth: usize,
        degree: usize,
        block_id: u32,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Vec<u32> {
        // get cur nodes
        let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
        let mut indirect_block_lock = cache.lock();
        let indirect_block = indirect_block_lock.as_mut_ref::<IndirectBlock>(0);

        let mut v = Vec::new();

        for i in 0..INDIRECT_CNT {
            if indirect_block[i] != 0 {
                v.push(indirect_block[i]);
                if depth < degree {
                    // not leaf node
                    v.extend(Self::dfs_clear_indirect(
                        depth + 1,
                        degree,
                        indirect_block[i],
                        block_device,
                    ));
                }
            }
            indirect_block[i] = 0;
        }
        v
    }
    pub fn valid_dir_entry_cnt(&self, block_device: &Arc<dyn BlockDevice>) -> usize {
        assert!(self.is_dir(), "Not a directory inode");
        let mut ret = 0;
        let mut dentry = DirEntry::empty();
        let file_count = self.size as usize / DIR_ENTRY_SIZE;
        for i in 0..file_count {
            assert_eq!(
                self.read_at(i * DIR_ENTRY_SIZE, dentry.as_bytes_mut(), block_device),
                DIR_ENTRY_SIZE,
            );
            if !dentry.is_empty() {
                ret += 1;
            }
        }
        ret
    }
}

pub const MAX_NAME_LENGTH: usize = 27;
// pub type DataBlock = [u8; BLOCK_SIZE];

#[repr(C)]
pub struct DataBlock(pub [u8; BLOCK_SIZE]);
impl DataBlock {
    pub fn clear(&mut self) {
        for byte in self.0.iter_mut() {
            *byte = 0;
        }
    }
}

#[repr(C)]
pub struct DirEntry {
    name: [u8; MAX_NAME_LENGTH + 1],
    inode_id: u32,
}

/// should be 32 bytes
pub const DIR_ENTRY_SIZE: usize = core::mem::size_of::<DirEntry>();
// pub const DIR_ENTRY_NUM: usize = BLOCK_SIZE / DIR_ENTRY_SIZE;

/// should be BLOCK_SIZE bytes
// pub type DirEntryBlock = [DirEntry; DIR_ENTRY_NUM];
// #[repr(C)]
// pub struct DirEntryBlock(pub [DirEntry; DIR_ENTRY_NUM]);

impl DirEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; MAX_NAME_LENGTH + 1],
            inode_id: 0,
        }
    }
    pub fn new(name: &str, inode_id: u32) -> Self {
        let mut new_name = [0u8; MAX_NAME_LENGTH + 1];
        let _ = &mut new_name[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: new_name,
            inode_id,
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIR_ENTRY_SIZE)
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIR_ENTRY_SIZE)
        }
    }
    pub fn name(&self) -> &str {
        let len = self
            .name
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(MAX_NAME_LENGTH - 1);
        core::str::from_utf8(&self.name[..len]).unwrap()
    }
    pub fn inode_id(&self) -> u32 {
        self.inode_id
    }
    pub fn is_empty(&self) -> bool {
        self.inode_id == 0 && self.name.iter().all(|&x| x == 0)
    }
}

// impl DirEntryBlock {
//     /// Find the inode inside a dir with given name
//     pub fn find_inode(&self, name: &str) -> Option<u32> {
//         self.0.iter().find_map(|entry| {
//             if entry.name() == name {
//                 Some(entry.inode_id())
//             } else {
//                 None
//             }
//         })
//     }
// }
