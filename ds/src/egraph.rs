use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::swap;
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
            let buffer_ref = unsafe { self.buffer_ptr.add(self.num_rows).as_mut().unwrap() };
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
            let row = unsafe { self.buffer_ptr.add(*row).as_ref().unwrap() };
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
                unsafe { self.table.buffer_ptr.add(self.row_idx).as_ref().unwrap() },
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

    fn delete_rows(&mut self, rows: &[usize]) {
        use SizeErasedTable::*;
        match self {
            OneOne(table) => table.delete_rows(rows),
            TwoOne(table) => table.delete_rows(rows),
            ThreeOne(table) => table.delete_rows(rows),
            FourOne(table) => table.delete_rows(rows),
        }
    }
}

fn canonicalize(uf: &mut UnionFind, det: &mut [u32], dep: &mut [u32], sig: Signature) -> bool {
    let mut changed = false;
    for class_id_col in sig.class_id_mask.iter_ones() {
        let col = if class_id_col < det.len() {
            &mut det[class_id_col]
        } else {
            &mut dep[class_id_col - det.len()]
        };
        let id = ClassId::from(*col);
        let canon = uf.find(id);
        changed = changed || id != canon;
        *col = u32::from(canon);
    }
    changed
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

    pub fn insert(&mut self, enode: &T) -> ClassId {
        const MAX_COLS: usize = 16;
        let mut encoded = [0u32; MAX_COLS];
        let sig = enode.signature();
        assert!(sig.num_det_cols + sig.num_dep_cols <= MAX_COLS);
        let (det, dep) =
            encoded[0..sig.num_det_cols + sig.num_dep_cols].split_at_mut(sig.num_det_cols);
        enode.encode_to_row(det, dep);
        canonicalize(&mut self.uf, det, dep, sig);
        let table = self
            .tables
            .entry(sig)
            .or_insert_with(|| SizeErasedTable::new(det.len(), dep.len()));
        Self::insert_or_merge(&mut self.uf, table, det, dep)
    }

    fn insert_or_merge(
        uf: &mut UnionFind,
        table: &mut SizeErasedTable,
        det: &[u32],
        dep: &[u32],
    ) -> ClassId {
        let old_root = ClassId::from(dep[0]);
        let new_dep = table.insert(det, dep);
        if let Some(new_dep) = new_dep {
            let new_root = ClassId::from(new_dep[0]);
            uf.merge(old_root, new_root);
            new_root
        } else {
            old_root
        }
    }

    pub fn makeset(&mut self) -> ClassId {
        self.uf.makeset()
    }

    pub fn find(&mut self, x: ClassId) -> ClassId {
        self.uf.find(x)
    }

    pub fn merge(&mut self, a: ClassId, b: ClassId) -> ClassId {
        self.uf.merge(a, b)
    }

    pub fn nodes(&self) -> impl Iterator<Item = T> + '_ {
        self.tables
            .iter()
            .map(|(sig, table)| {
                table
                    .rows()
                    .map(|row| T::decode_from_row(row.0, row.1, sig.clone()))
            })
            .flatten()
    }

    pub fn rebuild(&mut self) -> bool {
        let mut ever_changed = false;
        loop {
            let mut changed = false;
            let mut deleted_rows: Vec<usize> = vec![];
            let mut canonicalized_rows: Vec<u32> = vec![];

            for (sig, table) in &mut self.tables {
                deleted_rows.clear();
                canonicalized_rows.clear();

                for (det, dep, id) in table.rows() {
                    let before_len = canonicalized_rows.len();
                    canonicalized_rows.extend(det);
                    canonicalized_rows.extend(dep);
                    let row = &mut canonicalized_rows[before_len..];
                    let (det, dep) = row.split_at_mut(sig.num_det_cols);
                    if canonicalize(&mut self.uf, det, dep, *sig) {
                        changed = true;
                        deleted_rows.push(id);
                    } else {
                        canonicalized_rows.truncate(before_len);
                    }
                }

                table.delete_rows(&deleted_rows);

                let num_cols = sig.num_det_cols + sig.num_dep_cols;
                assert_eq!(canonicalized_rows.len(), deleted_rows.len() * num_cols);
                for idx in 0..deleted_rows.len() {
                    let det =
                        &canonicalized_rows[num_cols * idx..num_cols * idx + sig.num_det_cols];
                    let dep = &canonicalized_rows
                        [num_cols * idx + sig.num_det_cols..num_cols * (idx + 1)];
                    Self::insert_or_merge(&mut self.uf, table, det, dep);
                }
            }

            if !changed {
                break ever_changed;
            } else {
                ever_changed = true;
            }
        }
    }

    pub fn corebuild(&mut self) -> bool {
        let num_classes = self.uf.num_classes();
        let mut last_uf = UnionFind::new_all_equals(num_classes);
        let mut next_uf = UnionFind::new_all_not_equals(num_classes);

        loop {
            for (sig, table) in &mut self.tables {
                let mut before_roots: Vec<ClassId> = vec![];
                let mut canonicalized_rows: Vec<u32> = vec![];
                let mut enode_to_eclasses: HashMap<&[u32], Vec<ClassId>> = HashMap::new();

                for (det, dep, _) in table.rows() {
                    let before_len = canonicalized_rows.len();
                    canonicalized_rows.extend(det);
                    canonicalized_rows.extend(dep);
                    let row = &mut canonicalized_rows[before_len..];
                    let (det, dep) = row.split_at_mut(sig.num_det_cols);
                    before_roots.push(ClassId::from(dep[0]));
                    canonicalize(&mut last_uf, det, dep, *sig);
                }

                let num_cols = sig.num_det_cols + sig.num_dep_cols;
                for idx in 0..before_roots.len() {
                    let det =
                        &canonicalized_rows[idx * num_cols..idx * num_cols + sig.num_det_cols];
                    enode_to_eclasses
                        .entry(det)
                        .or_default()
                        .push(before_roots[idx]);
                }

                for equiv_classes in enode_to_eclasses.values() {
                    let head = equiv_classes[0];
                    for tail in &equiv_classes[1..] {
                        next_uf.merge(head, *tail);
                    }
                }
            }

            if last_uf == next_uf {
                break;
            } else {
                swap(&mut last_uf, &mut next_uf);
                next_uf.set_all_not_equals();
            }
        }

        let mut changed = false;
        for idx in 0..num_classes {
            let id = ClassId::from(idx);
            let canon = last_uf.find(id);
            changed = self.uf.find(id) != self.uf.find(canon) || changed;
            self.uf.merge(id, canon);
        }
        changed
    }

    pub fn full_repair(&mut self) -> bool {
        let mut changed = false;
        loop {
            changed = self.corebuild() || changed;
            if !self.rebuild() {
                break changed;
            } else {
                changed = true;
            }
        }
    }
}

impl<T: ENode + Debug> Debug for EGraph<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EGraph ({:?}) {{", self.uf)?;
        for (sig, table) in &self.tables {
            for row in table.rows() {
                write!(f, "{:?}, ", T::decode_from_row(&row.0, &row.1, *sig))?;
            }
        }
        write!(f, "}}")
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
