use alloc::sync::Arc;
use crate::mm::{
    frame_alloc, frame_dealloc, FrameGuard, PhysAddr, PhysPageNum, VirtAddr,
    KERNEL_MM,
};
use crate::sync::UPSafeCell;
use acore_fs::BlockDevice;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};
pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtIOHal>>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .exclusive_access()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}
#[allow(unused)]
const VIRTIO0: usize = 0x10001000;
impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtIOHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}

pub struct VirtIOHal;

lazy_static! {
    static ref DMA_FRAMES: UPSafeCell<Vec<FrameGuard>> = unsafe { UPSafeCell::new(Vec::new()) };
}

impl Hal for VirtIOHal {
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            DMA_FRAMES.exclusive_access().push(frame);
        }
        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let ppn_base: PhysPageNum = pa.into();
        for i in 0..pages {
            let ppn = PhysPageNum(ppn_base.0 + i);
            frame_dealloc(ppn);
        }
        0
    }

    fn phys_to_virt(pa: usize) -> usize {
        pa
    }

    fn virt_to_phys(va: usize) -> usize {
        KERNEL_MM
            .exclusive_access()
            .page_table
            .find_pa(VirtAddr::from(va))
            .unwrap()
            .0
    }
}


lazy_static!{
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(VirtIOBlock::new());
}