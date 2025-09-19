use std::collections::BTreeMap;

use ds::table::{Table, Value};

use crate::frontend::{Atom, Slot, Symbol};

pub type TableId = usize;

pub struct Database {
    tables: Vec<Table>,
    table_names: BTreeMap<Symbol, TableId>,
    scratch: Vec<Value>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            tables: vec![],
            table_names: BTreeMap::new(),
            scratch: vec![],
        }
    }

    pub fn register_table(&mut self, sym: Symbol, num_determinant: usize, num_dependent: usize) {
        assert!(!self.table_names.contains_key(&sym));
        let id = self.tables.len();
        self.tables.push(Table::new(num_determinant, num_dependent));
        self.table_names.insert(sym, id);
    }

    pub fn table_id(&self, sym: Symbol) -> TableId {
        self.table_names[&sym]
    }

    pub fn table(&self, id: TableId) -> &Table {
        &self.tables[id]
    }

    pub fn table_mut(&mut self, id: TableId) -> &mut Table {
        &mut self.tables[id]
    }

    pub fn insert_atom_with_subst(&mut self, atom: &Atom, subst: &BTreeMap<Symbol, Value>) {
        let table = &mut self.tables[atom.table];
        self.scratch.resize(table.num_determinant() + table.num_dependent(), 0);
        for (idx, slot) in atom.slots.iter().enumerate() {
            let value = match slot {
                Slot::Wildcard => panic!(),
                Slot::Variable(sym) => subst[&sym],
                Slot::Concrete(value) => *value,
            };
            self.scratch[idx] = value;
        }

    }
}
