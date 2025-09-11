use core::fmt::Debug;
use core::marker::PhantomData;
use std::collections::HashMap;

use bitvec::prelude::BitArray;
use memmap2::{MmapMut, MmapOptions};

use crate::uf::{ClassId, UnionFind};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature {
    pub class_id_mask: BitArray,
    pub num_det_cols: usize,
    pub num_dep_cols: usize,
    pub symbol_id: usize,
}

pub trait ENode {
    fn signature(&self) -> Signature;
    fn encode_to_row(&self, det: &mut [u32], dep: &mut [u32]);
    fn decode_from_row(det: &[u32], dep: &[u32], sig: Signature) -> Self;
}

const TABLE_VIRTUAL_ADDRESS_SIZE: usize = 1 << 40;

struct Table<const DET: usize, const DEP: usize> {
    num_rows: usize,
    _buffer: MmapMut,
    buffer_ptr: *mut ([u32; DET], [u32; DEP]),
    det_map: HashMap<&'static [u32; DET], &'static [u32; DEP]>,
    deleted_rows: Vec<usize>,
}

impl<const DET: usize, const DEP: usize> Table<DET, DEP> {
    fn new() -> Self {
        let mut buffer = MmapOptions::new()
            .no_reserve_swap()
            .len(TABLE_VIRTUAL_ADDRESS_SIZE)
            .map_anon()
            .unwrap();
        let buffer_ptr = unsafe {
            let buffer_aligned = buffer.align_to_mut::<([u32; DET], [u32; DEP])>().1;
            buffer_aligned.as_mut_ptr()
        };
        Self {
            num_rows: 0,
            _buffer: buffer,
            buffer_ptr,
            det_map: HashMap::new(),
            deleted_rows: vec![],
        }
    }

    fn insert(&mut self, det: [u32; DET], dep: [u32; DEP]) -> Option<&[u32; DEP]> {
        if let Some(new_dep) = self.det_map.get(&det) {
            Some(*new_dep)
        } else {
            let buffer_ref = unsafe { self.buffer_ptr.add(self.num_rows).as_mut_unchecked() };
            *buffer_ref = (det, dep);
            self.det_map.insert(&buffer_ref.0, &buffer_ref.1);
            self.num_rows += 1;
            None
        }
    }

    fn rows(&self) -> TableIterator<'_, DET, DEP> {
        TableIterator {
            table: self,
            row_idx: 0,
            deleted_rows_idx: 0,
        }
    }

    fn delete_rows(&mut self, rows: &[usize]) {
        for row in rows {
            let row = unsafe { self.buffer_ptr.add(*row).as_ref_unchecked() };
            self.det_map.remove(&row.0);
        }

        let mut merged = vec![];
        let mut old_idx = 0;
        let mut new_idx = 0;
        while old_idx < self.deleted_rows.len() && new_idx < rows.len() {
            if self.deleted_rows[old_idx] < rows[new_idx] {
                merged.push(self.deleted_rows[old_idx]);
                old_idx += 1;
            } else if rows[new_idx] < self.deleted_rows[old_idx] {
                merged.push(rows[new_idx]);
                new_idx += 1;
            } else {
                new_idx += 1;
            }
        }

        merged.extend(&self.deleted_rows[old_idx..]);
        merged.extend(&rows[new_idx..]);
        self.deleted_rows = merged;
    }
}

struct TableIterator<'a, const DET: usize, const DEP: usize> {
    table: &'a Table<DET, DEP>,
    row_idx: usize,
    deleted_rows_idx: usize,
}

impl<'a, const DET: usize, const DEP: usize> Iterator for TableIterator<'a, DET, DEP> {
    type Item = (&'a ([u32; DET], [u32; DEP]), usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.row_idx < self.table.num_rows {
            if self.deleted_rows_idx < self.table.deleted_rows.len() {
                assert!(self.row_idx <= self.table.deleted_rows[self.deleted_rows_idx]);
                if self.row_idx == self.table.deleted_rows[self.deleted_rows_idx] {
                    self.row_idx += 1;
                    self.deleted_rows_idx += 1;
                    continue;
                }
            }
            let item = (
                unsafe { self.table.buffer_ptr.add(self.row_idx).as_ref_unchecked() },
                self.row_idx,
            );
            self.row_idx += 1;
            return Some(item);
        }
        None
    }
}

enum SizeErasedTable {
    OneOne(Table<1, 1>),
    TwoOne(Table<2, 1>),
    ThreeOne(Table<3, 1>),
    FourOne(Table<4, 1>),
}

impl SizeErasedTable {
    fn new(num_det_cols: usize, num_dep_cols: usize) -> Self {
        use SizeErasedTable::*;
        match (num_det_cols, num_dep_cols) {
            (1, 1) => OneOne(Table::new()),
            (2, 1) => TwoOne(Table::new()),
            (3, 1) => ThreeOne(Table::new()),
            (4, 1) => FourOne(Table::new()),
            _ => todo!(),
        }
    }

    fn insert(&mut self, det: &[u32], dep: &[u32]) -> Option<&[u32]> {
        use SizeErasedTable::*;
        match self {
            OneOne(table) => table
                .insert(det.try_into().unwrap(), dep.try_into().unwrap())
                .map(|x| x as _),
            TwoOne(table) => table
                .insert(det.try_into().unwrap(), dep.try_into().unwrap())
                .map(|x| x as _),
            ThreeOne(table) => table
                .insert(det.try_into().unwrap(), dep.try_into().unwrap())
                .map(|x| x as _),
            FourOne(table) => table
                .insert(det.try_into().unwrap(), dep.try_into().unwrap())
                .map(|x| x as _),
        }
    }

    fn rows(&self) -> Box<dyn Iterator<Item = (&[u32], &[u32], usize)> + '_> {
        use SizeErasedTable::*;
        match self {
            OneOne(table) => Box::new(
                table
                    .rows()
                    .map(|((det, dep), id)| (det as _, dep as _, id)),
            ),
            TwoOne(table) => Box::new(
                table
                    .rows()
                    .map(|((det, dep), id)| (det as _, dep as _, id)),
            ),
            ThreeOne(table) => Box::new(
                table
                    .rows()
                    .map(|((det, dep), id)| (det as _, dep as _, id)),
            ),
            FourOne(table) => Box::new(
                table
                    .rows()
                    .map(|((det, dep), id)| (det as _, dep as _, id)),
            ),
        }
    }
}

pub struct EGraph<T: ENode> {
    tables: HashMap<Signature, SizeErasedTable>,
    uf: UnionFind,
    _phantom: PhantomData<T>,
}

impl<T: ENode> EGraph<T> {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            uf: UnionFind::new(),
            _phantom: PhantomData,
        }
    }

    fn canonicalize(&mut self, det: &mut [u32], dep: &mut [u32], sig: Signature) -> bool {
        let mut changed = false;
        for class_id_col in sig.class_id_mask.iter_ones() {
            let col = if class_id_col < det.len() {
                &mut det[class_id_col]
            } else {
                &mut dep[class_id_col - det.len()]
            };
            let id = ClassId::from(*col);
            let canon = self.uf.find(id);
            changed = changed || id != canon;
            *col = u32::from(canon);
        }
        changed
    }

    pub fn insert(&mut self, enode: &T) -> ClassId {
        const MAX_COLS: usize = 16;
        let mut encoded = [0u32; MAX_COLS];
        let sig = enode.signature();
        assert!(sig.num_det_cols + sig.num_dep_cols <= MAX_COLS);
        let (det, dep) =
            encoded[0..sig.num_det_cols + sig.num_dep_cols].split_at_mut(sig.num_det_cols);
        enode.encode_to_row(det, dep);
        self.canonicalize(det, dep, sig);
        let old_root = ClassId::from(dep[0]);
        let table = self
            .tables
            .entry(sig)
            .or_insert_with(|| SizeErasedTable::new(det.len(), dep.len()));
        let new_dep = table.insert(det, dep);
        if let Some(new_dep) = new_dep {
            let new_root = ClassId::from(new_dep[0]);
            self.uf.merge(old_root, new_root);
            new_root
        } else {
            old_root
        }
    }

    pub fn makeset(&mut self) -> ClassId {
        self.uf.makeset()
    }

    pub fn merge(&mut self, a: ClassId, b: ClassId) -> ClassId {
        self.uf.merge(a, b)
    }
}

impl<T: ENode + Debug> Debug for EGraph<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EGraph ({:?}):", self.uf)?;
        for (sig, table) in &self.tables {
            for row in table.rows() {
                writeln!(f, "{:?}", T::decode_from_row(&row.0, &row.1, *sig))?;
            }
        }
        Ok(())
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

        assert_eq!(mapping.len(), TABLE_VIRTUAL_ADDRESS_SIZE);
        for i in 0..(1 << 20) {
            mapping[i] = i as u8;
            mapping[i + (TABLE_VIRTUAL_ADDRESS_SIZE >> 1)] = i as u8;
        }
        for i in 0..(1 << 20) {
            assert_eq!(mapping[i], i as u8);
            assert_eq!(mapping[i + (TABLE_VIRTUAL_ADDRESS_SIZE >> 1)], i as u8);
        }

        let (_, word_slice, _) = unsafe { mapping.align_to::<u32>() };
        assert!(
            word_slice.len() == TABLE_VIRTUAL_ADDRESS_SIZE / 4
                || word_slice.len() == TABLE_VIRTUAL_ADDRESS_SIZE / 4 - 1
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn simple_egraph() {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Term {
            Constant(u32, ClassId),
            Add(ClassId, ClassId, ClassId),
        }

        use bitvec::bitarr;
        use bitvec::prelude::*;
        impl ENode for Term {
            fn signature(&self) -> Signature {
                match self {
                    Term::Constant(_, _) => Signature {
                        class_id_mask: bitarr![0, 1],
                        num_det_cols: 1,
                        num_dep_cols: 1,
                        symbol_id: 0,
                    },
                    Term::Add(_, _, _) => Signature {
                        class_id_mask: bitarr![1, 1, 1],
                        num_det_cols: 2,
                        num_dep_cols: 1,
                        symbol_id: 1,
                    },
                }
            }

            fn encode_to_row(&self, det: &mut [u32], dep: &mut [u32]) {
                match self {
                    Term::Constant(cons, root) => {
                        assert_eq!(det.len(), 1);
                        assert_eq!(dep.len(), 1);
                        det[0] = *cons;
                        dep[0] = u32::from(*root);
                    }
                    Term::Add(lhs, rhs, root) => {
                        assert_eq!(det.len(), 2);
                        assert_eq!(dep.len(), 1);
                        det[0] = u32::from(*lhs);
                        det[1] = u32::from(*rhs);
                        dep[0] = u32::from(*root);
                    }
                }
            }

            fn decode_from_row(det: &[u32], dep: &[u32], sig: Signature) -> Self {
                assert_eq!(det.len(), sig.num_det_cols);
                assert_eq!(dep.len(), sig.num_dep_cols);
                match sig.symbol_id {
                    0 => Term::Constant(det[0], ClassId::from(dep[0])),
                    1 => Term::Add(
                        ClassId::from(det[0]),
                        ClassId::from(det[1]),
                        ClassId::from(dep[0]),
                    ),
                    _ => todo!(),
                }
            }
        }

        let mut egraph = EGraph::new();
        let old_term1 = egraph.uf.makeset();
        let old_term2 = egraph.uf.makeset();
        let old_term3 = egraph.uf.makeset();
        let term1 = Term::Constant(5, old_term1);
        let term2 = Term::Constant(7, old_term2);
        let term3 = Term::Constant(7, old_term3);
        let term1 = egraph.insert(&term1);
        let term2 = egraph.insert(&term2);
        let term3 = egraph.insert(&term3);
        assert_eq!(term1, old_term1);
        assert_eq!(term2, old_term2);
        assert_ne!(term3, old_term3);
        assert_ne!(term1, term2);
        assert_eq!(term2, term3);
        let old_term4 = egraph.uf.makeset();
        let old_term5 = egraph.uf.makeset();
        let term4 = Term::Add(term1, old_term3, old_term4);
        let term5 = Term::Add(term1, term3, old_term5);
        let term4 = egraph.insert(&term4);
        let term5 = egraph.insert(&term5);
        assert_eq!(term4, old_term4);
        assert_ne!(term5, old_term5);
        assert_eq!(term4, term5);
    }
}
