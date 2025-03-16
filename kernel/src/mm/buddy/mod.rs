mod buddy_allocator;
mod buddy_list;

use alloc::format;
pub use buddy_allocator::Heap;
use crate::println;

#[allow(unused)]
pub fn test_vec() {
    use alloc::vec::Vec;
    use log::*;

    info!("Hello from test_vec");

    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let mut v: Vec<usize> = Vec::new();
    println!("sbss: {:x}, ebss: {:x}", sbss as usize, ebss as usize);
    println!("v.as_ptr(): {:x}", &(v.as_ptr() as usize));
    // assert!(bss_range.contains(&(v.as_ptr() as usize)));
    for i in 0..100000 {
        v.push(i);
    }
    println!("v.as_ptr(): {:x}", &(v.as_ptr() as usize));
    for (i, val) in v.iter().enumerate() {
        assert_eq!(*val, i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    info!("vec test passed!");
}

#[allow(unused)]
pub fn test_btree_map() {
    use alloc::collections::BTreeMap;
    use log::*;

    info!("Hello from test_btree_map");

    info!("Testing insert and get...");

    let mut map = BTreeMap::new();
    for i in 0..10000 {
        map.insert(i, format!("value{}", i));
    }
    for i in 0..10000 {
        assert_eq!(map.get(&i), Some(&format!("value{}", i)));
    }

    info!("Testing insert(update) and remove...");

    for i in 0..10000 {
        match i % 3 {
            0 => {
                map.insert(i, format!("ha{}ha{}", i, i));
            }
            1 => {
                map.insert(i, format!("hey{}hey{}", i, i));
            }
            2 => {
                assert_eq!(map.remove(&i), Some(format!("value{}", i)));
            }
            _ => {
                panic!("unreachable");
            }
        }
    }

    info!("Testing iteration...");

    for (i, s) in map.iter() {
        match i % 3 {
            0 => {
                assert_eq!(s, &format!("ha{}ha{}", i, i));
            }
            1 => {
                assert_eq!(s, &format!("hey{}hey{}", i, i));
            }
            _ => {
                panic!("unreachable");
            }
        }
    }

    while !map.is_empty() { map.pop_first(); }

    assert!(map.is_empty());

    info!("Testing repeatedly insert and remove...");
    for i in 0..10000 {
        map.insert(i, format!("value{}", i));
        map.remove(&i);
        assert!(map.is_empty());
    }
    for i in 0..5 {
        for j in 0..10000 {
            map.insert(j, format!("value{}", j));
        }
        map.clear();
        assert!(map.is_empty());
    }

    info!("btree_map test passed!");
}