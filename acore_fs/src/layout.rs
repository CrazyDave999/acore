use crate::block_manager::get_block_cache;
use crate::{BlockDevice, BLOCK_SIZE};
use alloc::sync::Arc;
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
    type_: DiskInodeType,
}

pub const DISK_INODE_SIZE: usize = core::mem::size_of::<DiskInode>();
pub const DISK_INODE_PER_BLOCK: usize = BLOCK_SIZE / DISK_INODE_SIZE;

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
    pub fn is_file(&self) -> bool {
        matches!(self.type_, DiskInodeType::File)
    }

    /// Get block_id with given inner_id
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        let end_id = INODE_DIRECT_CNT + {
            let mut res = 0;
            for degree in 0..MAX_INDIRECT_DEGREE {
                res += INDIRECT_CNT.pow((degree + 1) as u32);
            }
            res
        };

        if inner_id < INODE_DIRECT_CNT {
            self.direct[inner_id]
        } else if inner_id < end_id {
            let mut inner_id = inner_id - INODE_DIRECT_CNT;
            // cur total end id for this degree's indirect block can manage
            for (degree, &root_id) in self.indirect.iter().enumerate() {
                let cur_end_id = INDIRECT_CNT.pow((degree + 1) as u32);
                if inner_id < cur_end_id {
                    let mut block_id = root_id;
                    for i in (1..degree + 1).rev() {
                        let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
                        block_id = cache.lock().as_ref::<IndirectBlock>(0)
                            [inner_id / INDIRECT_CNT.pow(i as u32)];
                        inner_id %= INDIRECT_CNT.pow(i as u32);
                    }
                    let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
                    block_id = cache.lock().as_ref::<IndirectBlock>(0)[inner_id % INDIRECT_CNT];
                    return block_id;
                }
                if degree == MAX_INDIRECT_DEGREE - 1 {
                    panic!("Should not reach here");
                }
                inner_id -= cur_end_id;
            }
            panic!("Should not reach here");
        } else {
            panic!("File too large");
        }
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
        assert!(end > start);
        let mut start_inner_block_id = start / BLOCK_SIZE;
        let mut read_size = 0usize;
        loop {
            let mut end_current_block = min(end, (start / BLOCK_SIZE + 1) * BLOCK_SIZE);
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            let block_id = self.get_block_id(start_inner_block_id as u32, block_device);
            let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
            let data_block = cache.lock().as_ref::<DataBlock>(0);
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
        let mut start = offset;
        let end = min(offset + buf.len(), self.size as usize);
        assert!(end > start);
        let mut start_inner_block_id = start / BLOCK_SIZE;
        let mut write_size = 0usize;
        loop {
            let mut end_current_block = min(end, (start / BLOCK_SIZE + 1) * BLOCK_SIZE);
            let block_write_size = end_current_block - start;
            let block_id = self.get_block_id(start_inner_block_id as u32, block_device);
            let cache = get_block_cache(block_id as usize, Arc::clone(block_device));
            let data_block = cache.lock().as_mut_ref::<DataBlock>(0);
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

    /// increase size of current inode, with given new data blocks
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        todo!()
    }

    /// clear size of current inode, return the data blocks
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        todo!()
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
pub const DIR_ENTRY_NUM: usize = BLOCK_SIZE / DIR_ENTRY_SIZE;

/// should be BLOCK_SIZE bytes
// pub type DirEntryBlock = [DirEntry; DIR_ENTRY_NUM];
#[repr(C)]
pub struct DirEntryBlock(pub [DirEntry; DIR_ENTRY_NUM]);

impl DirEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; MAX_NAME_LENGTH + 1],
            inode_id: 0,
        }
    }
    pub fn new(name: &str, inode_id: u32) -> Self {
        let mut new_name = [0u8; MAX_NAME_LENGTH + 1];
        &mut new_name[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: new_name,
            inode_id,
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::mem::transmute(self) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::mem::transmute(self) }
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
