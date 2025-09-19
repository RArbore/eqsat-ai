use std::collections::HashMap;

use ds::table::Table;

use crate::frontend::Symbol;

pub type TableId = usize;

pub struct Database {
    pub(crate) tables: Vec<Table>,
    pub(crate) table_names: HashMap<Symbol, TableId>,
}

impl Database {
    pub(crate) fn new() -> Self {
        Self {
            tables: vec![],
            table_names: HashMap::new(),
        }
    }

    pub(crate) fn register_table(&mut self, sym: Symbol, num_determinant: usize, num_dependent: usize) {
        let id = self.tables.len();
        self.tables.push(Table::new(num_determinant, num_dependent));
        self.table_names.insert(sym, id);
    }

    pub(crate) fn table_id(&self, sym: Symbol) -> TableId {
        self.table_names[&sym]
    }
}
