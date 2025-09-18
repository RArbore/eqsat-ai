use core::hash::Hasher;
use std::collections::BTreeSet;
use std::collections::btree_set::Iter;
use std::iter::Peekable;

use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use rustc_hash::FxHasher;

pub type Value = u32;
type RowId = u64;
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
}

#[derive(Debug)]
struct TableRows<'a> {
    table: &'a Table,
    row: RowId,
    deleted_iter: Peekable<Iter<'a, RowId>>,
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
    pub fn new(num_determinant: usize, num_dependent: usize) -> Table {
        Table {
            rows: Rows {
                buffer: vec![],
                num_determinant,
                num_dependent,
            },
            table: HashTable::new(),
            deleted_rows: BTreeSet::new(),
        }
    }

    fn num_determinant(&self) -> usize {
        self.rows.num_determinant
    }

    fn num_dependent(&self) -> usize {
        self.rows.num_dependent
    }

    pub fn insert(&mut self, row: &[Value]) -> (&[Value], RowId) {
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

    pub fn rows(&self) -> impl Iterator<Item = &[Value]> + '_ {
        TableRows {
            table: self,
            row: 0,
            deleted_iter: self.deleted_rows.iter().peekable(),
        }
    }
}

impl<'a> Iterator for TableRows<'a> {
    type Item = &'a [Value];

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
            Some(self.table.rows.get_row(row))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_table() {
        let mut table = Table::new(2, 1);
        assert_eq!(table.insert(&[1, 2, 3]), (&[1u32, 2, 3] as _, 0));
        assert_eq!(table.insert(&[1, 2, 4]), (&[1u32, 2, 3] as _, 0));
        assert_eq!(table.insert(&[2, 2, 4]), (&[2u32, 2, 4] as _, 1));
        assert_eq!(vec![&[1, 2, 3], &[2, 2, 4]], table.rows().collect::<Vec<_>>());
        assert_eq!(table.delete(1), &[2, 2, 4]);
        assert_eq!(table.insert(&[2, 2, 5]), (&[2u32, 2, 5] as _, 2));
        assert_eq!(vec![&[1, 2, 3], &[2, 2, 5]], table.rows().collect::<Vec<_>>());
    }
}
