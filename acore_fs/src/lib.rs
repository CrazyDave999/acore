pub use crate::block_dev::BlockDevice;

mod block_cache;
mod block_dev;
mod block_manager;
mod layout;
mod bitmap;
mod afs;
mod vfs;

extern crate alloc;
extern crate lru;
pub const BLOCK_SIZE: usize = 512;
pub use vfs::Inode;
pub use afs::AcoreFileSystem;
pub use layout::DiskInodeType;
