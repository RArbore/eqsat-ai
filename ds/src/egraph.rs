use core::marker::PhantomData;
use core::mem::size_of;
use core::slice::from_raw_parts_mut;
use std::collections::{BTreeSet, HashMap};

use bincode::config::legacy;
use bincode::{Decode, Encode, decode_from_slice, encode_into_slice};
use memmap2::{MmapMut, MmapOptions};

use crate::uf::{ClassId, UnionFind};

pub trait ENode: Copy + Eq + Encode + Decode<()> {
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
    deleted_rows: BTreeSet<u32>,
}

pub struct EGraph<T: ENode> {
    tables: HashMap<&'static str, Table>,

    uf: UnionFind,
    _phantom: PhantomData<T>,
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
            deleted_rows: BTreeSet::new(),
        }
    }

    fn insert(&mut self, enode: &[u8]) -> Option<&'static [u8]> {
        assert_eq!(
            self.num_bytes_det + self.num_bytes_dep,
            enode.len().try_into().unwrap()
        );
        let det = &enode[0..self.num_bytes_det as usize];
        if let Some(dep) = self.det_map.get(det) {
            Some(dep)
        } else {
            let offset = (self.num_rows * (self.num_bytes_det + self.num_bytes_dep)) as usize;
            let buffer_slice =
                unsafe { from_raw_parts_mut(self.buffer.as_mut_ptr().add(offset), enode.len()) };
            buffer_slice.copy_from_slice(enode);
            let det = &buffer_slice[0..self.num_bytes_det as usize];
            let dep = &buffer_slice[self.num_bytes_det as usize..];
            self.det_map.insert(det, dep);
            None
        }
    }

    fn delete_row(&mut self, row: u32) {
        let offset = (row * (self.num_bytes_det + self.num_bytes_dep)) as usize;
        let det = &self.buffer[offset..offset+self.num_bytes_det as usize];
        self.det_map.remove(det);
        self.deleted_rows.insert(row);
    }
}

impl<T: ENode> EGraph<T> {
    pub fn new() -> Self {
        Self {
            uf: UnionFind::new(),
            tables: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, mut enode: T) -> ClassId {
        enode.canonicalize(&mut self.uf);
        let old_root = enode.root();
        const MAX_ENCODED_SIZE: usize = 64;
        let mut enode_buf = [0u8; MAX_ENCODED_SIZE];
        let size = encode_into_slice(enode, &mut enode_buf, legacy()).unwrap() as u32;
        let table = self
            .tables
            .entry(enode.function_symbol())
            .or_insert_with(|| Table::new(size - size_of::<ClassId>() as u32));
        assert_eq!(table.num_bytes_det + table.num_bytes_dep, size);
        let enode_buf = &mut enode_buf[..size as usize];
        let new_dep = table.insert(enode_buf);
        if let Some(new_dep) = new_dep {
            let old_dep = &mut enode_buf[table.num_bytes_det as usize..];
            old_dep.copy_from_slice(new_dep);
            let new_enode: T = decode_from_slice(enode_buf, legacy()).unwrap().0;
            let new_root = new_enode.root();
            self.uf.merge(old_root, new_root);
            new_root
        } else {
            old_root
        }
    }

    pub fn merge(&mut self, a: ClassId, b: ClassId) -> ClassId {
        self.uf.merge(a, b)
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

    #[test]
    #[cfg_attr(miri, ignore)]
    fn simple_egraph() {
        #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
        enum Term {
            Constant(i32, ClassId),
            Add(ClassId, ClassId, ClassId),
        }

        impl ENode for Term {
            fn root(&self) -> ClassId {
                match self {
                    Term::Constant(_, root) | Term::Add(_, _, root) => *root,
                }
            }

            fn canonicalize(&mut self, uf: &mut UnionFind) {
                match self {
                    Term::Constant(_, root) => {
                        *root = uf.find(*root);
                    },
                    Term::Add(lhs, rhs, root) => {
                        *lhs = uf.find(*lhs);
                        *rhs = uf.find(*rhs);
                        *root = uf.find(*root);
                    }
                }
            }

            fn function_symbol(&self) -> &'static str {
                match self {
                    Term::Constant(_, _) => "cons",
                    Term::Add(_, _, _) => "add",
                }
            }
        }

        let mut egraph = EGraph::new();
        let term1 = Term::Constant(5, egraph.uf.makeset());
        let term2 = Term::Constant(7, egraph.uf.makeset());
        let term3 = Term::Constant(7, egraph.uf.makeset());
        let term1 = egraph.insert(term1);
        let term2 = egraph.insert(term2);
        let term3 = egraph.insert(term3);
        assert_ne!(term1, term2);
        assert_eq!(term2, term3);
        let term4 = Term::Add(term1, term2, egraph.uf.makeset());
        let term5 = Term::Add(term1, term2, egraph.uf.makeset());
        let term4 = egraph.insert(term4);
        let term5 = egraph.insert(term5);
        assert_eq!(term4, term5);
    }
}
