use core::cell::RefCell;
use core::fmt::Debug;
use std::collections::BTreeMap;

use ds::table::{CanonFn, Canonizer, MergeFn, Merger, Table, Value, rebuild};
use ds::uf::{ClassId, UnionFind};

use crate::frontend::{Atom, Schema, SchemaColumn, Slot, Symbol};

pub type TableId = usize;

pub struct Database<'a> {
    tables: Vec<Table>,
    schemas: Vec<Schema>,
    mergers: Vec<Merger<'a>>,
    canonizers: Vec<Canonizer<'a>>,
    table_names: BTreeMap<Symbol, TableId>,
    scratch: RefCell<Vec<Value>>,
    aux_state: DatabaseAuxiliaryState<'a>,
}

#[derive(Clone, Debug)]
pub struct DatabaseAuxiliaryState<'a> {
    pub uf: &'a UnionFind,
}

impl<'a> Database<'a> {
    pub fn new(aux_state: DatabaseAuxiliaryState<'a>) -> Self {
        Self {
            tables: vec![],
            schemas: vec![],
            mergers: vec![],
            canonizers: vec![],
            table_names: BTreeMap::new(),
            scratch: RefCell::new(vec![]),
            aux_state,
        }
    }

    pub fn register_table(&mut self, sym: Symbol, schema: Schema) {
        assert!(!self.table_names.contains_key(&sym));
        let id = self.tables.len();
        let num_determinant = schema.determinant.len();
        let num_dependent = schema.dependent.len();
        self.tables.push(Table::new(num_determinant, num_dependent));
        self.schemas.push(schema.clone());

        let aux_state = self.aux_state.clone();
        let other_schema = schema.clone();
        let other_aux_state = aux_state.clone();
        let merger = Box::new(move |a: &[Value], b: &[Value], dst: &mut [Value]| {
            default_merger(&schema, aux_state.clone(), a, b, dst)
        });
        let canonizer = Box::new(move |x: &[Value], dst: &mut [Value]| {
            default_canonizer(&other_schema, other_aux_state.clone(), x, dst)
        });
        self.mergers
            .push(Merger::new(num_determinant + num_dependent, merger));
        self.canonizers
            .push(Canonizer::new(num_determinant + num_dependent, canonizer));

        self.table_names.insert(sym, id);
    }

    pub fn register_custom_table(
        &mut self,
        sym: Symbol,
        schema: Schema,
        merger: MergeFn<'a>,
        canonizer: CanonFn<'a>,
    ) {
        assert!(!self.table_names.contains_key(&sym));
        let id = self.tables.len();
        let num_determinant = schema.determinant.len();
        let num_dependent = schema.dependent.len();
        self.tables.push(Table::new(num_determinant, num_dependent));
        self.schemas.push(schema.clone());
        self.mergers
            .push(Merger::new(num_determinant + num_dependent, merger));
        self.canonizers
            .push(Canonizer::new(num_determinant + num_dependent, canonizer));
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

    pub fn schema(&self, id: TableId) -> &Schema {
        &self.schemas[id]
    }

    pub fn insert_atom_with_subst(&mut self, atom: &Atom, subst: &BTreeMap<Symbol, Value>) -> bool {
        let table = &mut self.tables[atom.table];
        let mut scratch = self.scratch.borrow_mut();
        scratch.resize(atom.slots.len(), 0);
        for (idx, slot) in atom.slots.iter().enumerate() {
            let value = match slot {
                Slot::Wildcard => panic!(),
                Slot::Variable(sym) => {
                    subst[&sym]
                }
                Slot::Concrete(value) => *value,
            };
            scratch[idx] = value;
        }
        let merge = &mut self.mergers[atom.table];
        merge.insert(table, &scratch).1
    }

    pub fn get_with_subst(&self, atom: &Atom, subst: &BTreeMap<Symbol, Value>) -> Option<&[Value]> {
        let table = &self.tables[atom.table];
        let mut scratch = self.scratch.borrow_mut();
        scratch.resize(table.num_determinant(), 0);
        for idx in 0..table.num_determinant() {
            let value = match atom.slots[idx] {
                Slot::Wildcard => panic!(),
                Slot::Variable(sym) => subst[&sym],
                Slot::Concrete(value) => value,
            };
            scratch[idx] = value;
        }
        table.get(&scratch)
    }

    pub fn repair(&mut self) -> bool {
        let mut ever_changed = false;
        loop {
            let mut changed = false;
            for id in 0..self.tables.len() {
                changed = rebuild(
                    &mut self.tables[id],
                    &mut self.mergers[id],
                    &mut self.canonizers[id],
                ) || changed;
            }
            if !changed {
                break ever_changed;
            } else {
                ever_changed = true;
            }
        }
    }

    pub fn aux_state(&self) -> &DatabaseAuxiliaryState<'a> {
        &self.aux_state
    }
}

impl<'a> Debug for Database<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("tables", &self.tables)
            .finish()
    }
}

fn default_merger(
    schema: &Schema,
    aux_state: DatabaseAuxiliaryState<'_>,
    a: &[Value],
    b: &[Value],
    dst: &mut [Value],
) {
    let num_determinant = schema.determinant.len();
    for (idx, column) in schema.dependent.iter().enumerate() {
        let idx = idx + num_determinant;
        use SchemaColumn::*;
        match column {
            EClassId => {
                dst[idx] = aux_state
                    .uf
                    .merge(ClassId::from(a[idx]), ClassId::from(b[idx]))
                    .into()
            }
            Int => {
                assert_eq!(a[idx], b[idx]);
                dst[idx] = a[idx];
            }
            _ => panic!(),
        }
    }
}

fn default_canonizer(
    schema: &Schema,
    aux_state: DatabaseAuxiliaryState<'_>,
    x: &[Value],
    dst: &mut [Value],
) {
    for (idx, column) in schema
        .determinant
        .iter()
        .chain(schema.dependent.iter())
        .enumerate()
    {
        use SchemaColumn::*;
        match column {
            EClassId => dst[idx] = aux_state.uf.find(ClassId::from(x[idx])).into(),
            Int => dst[idx] = x[idx],
            _ => panic!(),
        }
    }
}
