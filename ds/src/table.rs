use core::hash::Hasher;
use std::collections::BTreeSet;
use std::collections::btree_set::Iter;
use std::iter::Peekable;
use std::slice::from_raw_parts;

use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use rustc_hash::FxHasher;

pub type Value = u32;
pub type RowId = u64;
type HashCode = u64;

#[derive(Debug)]
struct TableEntry {
    hash: HashCode,
    row: RowId,
}

#[derive(Debug)]
struct Rows {
    buffer: Vec<Value>,
    num_determinant: usize,
    num_dependent: usize,
}

#[derive(Debug)]
pub struct Table {
    rows: Rows,
    table: HashTable<TableEntry>,
    deleted_rows: BTreeSet<RowId>,
    delta_marker: RowId,
}

#[derive(Debug)]
struct TableRows<'a> {
    table: &'a Table,
    row: RowId,
    deleted_iter: Peekable<Iter<'a, RowId>>,
}

pub struct Merger<MF>
where
    MF: FnMut(&[Value], &[Value], &mut [Value]),
{
    merge_fn: MF,
    scratch: Vec<Value>,
}

pub struct Canonizer<CF>
where
    CF: FnMut(&[Value], &mut [Value]),
{
    canon_fn: CF,
    scratch: Vec<Value>,
}

fn hash(determinant: &[Value]) -> HashCode {
    let mut hasher = FxHasher::default();
    for val in determinant {
        hasher.write_u32(*val);
    }
    hasher.finish()
}

impl Rows {
    fn num_rows(&self) -> RowId {
        let num_columns = self.num_determinant + self.num_dependent;
        (self.buffer.len() / num_columns) as RowId
    }

    fn get_row(&self, row: RowId) -> &[Value] {
        let num_columns = self.num_determinant + self.num_dependent;
        let start = (row as usize) * num_columns;
        &self.buffer[start..start + num_columns]
    }

    fn add_row(&mut self, row: &[Value]) -> RowId {
        let row_id = self.num_rows();
        self.buffer.extend(row);
        row_id
    }
}

impl Table {
    pub fn new(num_determinant: usize, num_dependent: usize) -> Self {
        Self {
            rows: Rows {
                buffer: vec![],
                num_determinant,
                num_dependent,
            },
            table: HashTable::new(),
            deleted_rows: BTreeSet::new(),
            delta_marker: 0,
        }
    }

    pub fn num_determinant(&self) -> usize {
        self.rows.num_determinant
    }

    pub fn num_dependent(&self) -> usize {
        self.rows.num_dependent
    }

    pub fn mark_delta(&mut self) {
        self.delta_marker = self.rows.num_rows();
    }

    pub fn insert<'a, 'b>(&'a mut self, row: &'b [Value]) -> (&'a [Value], RowId) {
        let num_determinant = self.num_determinant();
        let num_dependent = self.num_dependent();
        assert_eq!(row.len(), num_determinant + num_dependent);
        let determinant = &row[0..num_determinant];
        let hash = hash(determinant);
        let entry = self.table.entry(
            hash,
            |te| te.hash == hash && &self.rows.get_row(te.row)[0..num_determinant] == determinant,
            |te| te.hash,
        );
        match entry {
            Entry::Occupied(occupied) => {
                let row = occupied.get().row;
                (self.rows.get_row(row), row)
            }
            Entry::Vacant(vacant) => {
                let row = self.rows.add_row(row);
                vacant.insert(TableEntry { hash, row });
                (self.rows.get_row(row), row)
            }
        }
    }

    pub fn delete(&mut self, row_id: RowId) -> &[Value] {
        let row = self.rows.get_row(row_id);
        let determinant = &row[0..self.num_determinant()];
        let hash = hash(determinant);
        let entry = self
            .table
            .entry(hash, |te| te.hash == hash && te.row == row_id, |te| te.hash);
        let Entry::Occupied(occupied) = entry else {
            panic!();
        };
        occupied.remove();
        self.deleted_rows.insert(row_id);
        row
    }

    pub fn rows(&self, delta: bool) -> impl Iterator<Item = (&[Value], RowId)> + '_ {
        TableRows {
            table: self,
            row: if delta { self.delta_marker } else { 0 },
            deleted_iter: self.deleted_rows.iter().peekable(),
        }
    }
}

impl<'a> Iterator for TableRows<'a> {
    type Item = (&'a [Value], RowId);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(recent_deleted) = self.deleted_iter.peek() {
            if **recent_deleted > self.row {
                break;
            } else if **recent_deleted == self.row {
                self.row += 1;
            }
            self.deleted_iter.next();
        }

        if self.row >= self.table.rows.num_rows() {
            None
        } else {
            let row = self.row;
            self.row += 1;
            Some((self.table.rows.get_row(row), row))
        }
    }
}

impl<MF> Merger<MF>
where
    MF: FnMut(&[Value], &[Value], &mut [Value]),
{
    pub fn new(num_columns: usize, merge_fn: MF) -> Self {
        Self {
            merge_fn,
            scratch: vec![0; num_columns],
        }
    }

    pub fn insert<'a, 'b, 'c>(&'a mut self, table: &'b mut Table, row: &'c [Value]) -> &'b [Value] {
        let num_determinant = table.num_determinant();
        let would_be_new_id = table.rows.num_rows();
        let (in_row, row_id) = table.insert(row);
        if row_id == would_be_new_id {
            let in_row = &in_row[num_determinant..];
            return unsafe { from_raw_parts(in_row.as_ptr(), in_row.len()) };
        }
        self.scratch.copy_from_slice(row);
        (self.merge_fn)(
            &row,
            &in_row,
            &mut self.scratch,
        );
        if &in_row[num_determinant..] == &mut self.scratch[num_determinant..] {
            let in_row = &in_row[num_determinant..];
            return unsafe { from_raw_parts(in_row.as_ptr(), in_row.len()) };
        }
        table.delete(row_id);
        table.insert(&self.scratch).0
    }
}

impl<CF> Canonizer<CF>
where
    CF: FnMut(&[Value], &mut [Value]),
{
    pub fn new(num_columns: usize, canon_fn: CF) -> Self {
        Self {
            canon_fn,
            scratch: vec![0; num_columns],
        }
    }

    pub fn canon<'a, 'b>(&'a mut self, row: &'b [Value]) -> Option<&'a [Value]> {
        (self.canon_fn)(row, &mut self.scratch);
        if self.scratch == row {
            None
        } else {
            Some(&self.scratch)
        }
    }
}

pub fn rebuild<CF, MF>(table: &mut Table, mf: MF, cf: CF) -> bool
where
    MF: FnMut(&[Value], &[Value], &mut [Value]),
    CF: FnMut(&[Value], &mut [Value]),
{
    let num_columns = table.num_determinant() + table.num_dependent();
    let mut canonizer = Canonizer::new(num_columns, cf);
    let mut merger = Merger::new(num_columns, mf);
    let mut canonized: Vec<Value> = vec![];
    let mut to_delete: Vec<RowId> = vec![];
    let mut ever_changed = false;
    loop {
        let mut changed = false;

        for (row, row_id) in table.rows(false) {
            if let Some(canon_row) = canonizer.canon(row) {
                changed = true;
                canonized.extend(canon_row);
                to_delete.push(row_id);
            }
        }

        for row in to_delete.drain(..) {
            table.delete(row);
        }

        let num_rows = canonized.len() / num_columns;
        for idx in 0..num_rows {
            let canon_row = &canonized[idx * num_columns..(idx + 1) * num_columns];
            merger.insert(table, canon_row);
        }
        canonized.clear();

        if !changed {
            break ever_changed;
        } else {
            ever_changed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::min;

    use crate::uf::{ClassId, UnionFind};

    use super::*;

    #[test]
    fn simple_table() {
        let mut table = Table::new(2, 1);
        assert_eq!(table.insert(&[1, 2, 3]), (&[1u32, 2, 3] as _, 0));
        assert_eq!(table.insert(&[1, 2, 4]), (&[1u32, 2, 3] as _, 0));
        assert_eq!(table.insert(&[2, 2, 4]), (&[2u32, 2, 4] as _, 1));
        assert_eq!(
            vec![(&[1u32, 2, 3] as _, 0), (&[2u32, 2, 4] as _, 1)],
            table.rows(false).collect::<Vec<_>>()
        );
        assert_eq!(table.delete(1), &[2, 2, 4]);
        assert_eq!(table.insert(&[2, 2, 5]), (&[2u32, 2, 5] as _, 2));
        assert_eq!(
            vec![(&[1u32, 2, 3] as _, 0), (&[2u32, 2, 5] as _, 2)],
            table.rows(false).collect::<Vec<_>>()
        );
    }

    #[test]
    fn simple_merge() {
        let mut table = Table::new(2, 1);
        let mut merger = Merger::new(3, |a, b, dst| dst[2] = min(a[2], b[2]));
        merger.insert(&mut table, &[1, 2, 5]);
        merger.insert(&mut table, &[1, 2, 3]);
        merger.insert(&mut table, &[2, 2, 7]);
        merger.insert(&mut table, &[2, 2, 9]);
        merger.insert(&mut table, &[1, 2, 4]);
        assert_eq!(
            vec![(&[1u32, 2, 3] as _, 1), (&[2u32, 2, 7] as _, 2)],
            table.rows(false).collect::<Vec<_>>()
        );
        assert_eq!(table.rows.num_rows(), 3);
    }

    #[test]
    fn simple_canon() {
        let mut canonizer = Canonizer::new(1, |x, dst| dst[0] = (x[0] >> 1) << 1);
        let row = &[3];
        assert_eq!(canonizer.canon(row), Some(&[2u32] as _));
        let row = &[4];
        assert_eq!(canonizer.canon(row), None);
    }

    #[test]
    fn simple_rebuild() {
        let mut table = Table::new(1, 1);
        let mut uf = UnionFind::new();

        let id1 = uf.makeset();
        let id2 = uf.makeset();
        let id3 = uf.makeset();
        let id4 = uf.makeset();

        table.insert(&[id1.into(), id2.into()]);
        table.insert(&[id3.into(), id4.into()]);
        assert_eq!(
            vec![(&[0u32, 1] as _, 0), (&[2u32, 3] as _, 1)],
            table.rows(false).collect::<Vec<_>>()
        );

        uf.merge(id1, id3);
        rebuild(
            &mut table,
            |lhs, rhs, dst| {
                dst[1] = uf
                    .merge(ClassId::from(lhs[1]), ClassId::from(rhs[1]))
                    .into()
            },
            |x, dst| {
                dst[0] = uf.find(ClassId::from(x[0])).into();
                dst[1] = uf.find(ClassId::from(x[1])).into();
            },
        );

        assert_eq!(
            vec![(&[0u32, 1] as _, 0)],
            table.rows(false).collect::<Vec<_>>()
        );
    }

    #[test]
    fn simple_delta() {
        let mut table = Table::new(1, 1);
        table.insert(&[0, 1]);
        table.insert(&[1, 2]);
        assert_eq!(
            vec![(&[0u32, 1] as _, 0), (&[1u32, 2] as _, 1)],
            table.rows(true).collect::<Vec<_>>()
        );
        table.mark_delta();
        table.insert(&[2, 3]);
        assert_eq!(
            vec![(&[2u32, 3] as _, 2)],
            table.rows(true).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![(&[0u32, 1] as _, 0), (&[1u32, 2] as _, 1), (&[2u32, 3] as _, 2)],
            table.rows(false).collect::<Vec<_>>()
        );
    }
}
