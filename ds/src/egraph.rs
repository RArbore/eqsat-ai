use core::mem::size_of;
use std::collections::HashMap;

use bincode::{Decode, Encode};
use memmap2::{MmapMut, MmapOptions};

use crate::uf::{ClassId, UnionFind};

pub trait ENode: Copy + Encode + Decode<()> {
    fn root(&self) -> ClassId;
    fn canonicalize(&mut self, uf: &mut UnionFind);
    fn function_symbol(&self) -> &'static str;
}

const TABLE_VIRTUAL_ADDRESS_SIZE: usize = 1 << 40;

struct Table {
    num_bytes_det: u32,
    num_bytes_dep: u32,
    num_rows: u32,

    buffer: MmapMut,
    det_map: HashMap<&'static [u8], &'static [u8]>,
}

pub struct EGraph {
    uf: UnionFind,

    tables: HashMap<&'static str, Table>,
}

impl Table {
    fn new(num_bytes_det: u32) -> Self {
        Self {
            num_bytes_det,
            num_bytes_dep: size_of::<ClassId>() as u32,
            num_rows: 0,
            buffer: MmapOptions::new()
                .no_reserve_swap()
                .len(TABLE_VIRTUAL_ADDRESS_SIZE)
                .map_anon()
                .unwrap(),
            det_map: HashMap::new(),
        }
    }

    fn insert(&mut self, enode: &[u8]) -> &'static [u8] {
        assert_eq!(self.num_bytes_det + self.num_bytes_dep, enode.len().try_into().unwrap());
        todo!()
    }
}

impl EGraph {
    pub fn new() -> Self {
        Self {
            uf: UnionFind::new(),
            tables: HashMap::new(),
        }
    }

    pub fn insert<T: ENode>(&mut self, mut enode: T) -> ClassId {
        enode.canonicalize(&mut self.uf);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn mmap_works() {
        let mut mapping = MmapOptions::new()
            .no_reserve_swap()
            .len(TABLE_VIRTUAL_ADDRESS_SIZE)
            .map_anon()
            .unwrap();
        for i in 0..(1 << 20) {
            mapping[i] = i as u8;
            mapping[i + (TABLE_VIRTUAL_ADDRESS_SIZE >> 1)] = i as u8;
        }
        for i in 0..(1 << 20) {
            assert_eq!(mapping[i], i as u8);
            assert_eq!(mapping[i + (TABLE_VIRTUAL_ADDRESS_SIZE >> 1)], i as u8);
        }
    }
}
