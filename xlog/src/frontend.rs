use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

use ds::table::Value;

use crate::database::{Database, TableId};
use crate::fixpoint::ComputeFn;

pub type Symbol = SymbolU16;
pub type Interner = StringInterner<StringBackend<Symbol>>;

#[derive(Clone, Copy, Debug)]
pub enum Slot {
    Wildcard,
    Variable(Symbol),
    Concrete(Value),
}

#[derive(Clone, Debug)]
pub struct Atom {
    pub table: TableId,
    pub slots: Vec<Slot>,
}

#[derive(Clone, Debug)]
pub struct Query {
    pub atoms: Vec<Atom>,
}

pub enum Action {
    InsertPattern {
        atoms: Vec<Atom>,
    },
    ComputeFunc {
        func: ComputeFn,
        next: Box<Action>,
    },
}

pub struct Rule {
    pub query: Query,
    pub action: Action,
}

#[derive(Clone, Debug)]
pub struct Schema {
    pub determinant: Vec<SchemaColumn>,
    pub dependent: Vec<SchemaColumn>,
}

#[derive(Clone, Debug)]
pub enum SchemaColumn {
    EClassId,
    Symbol,
    Int,
    CustomLattice,
}

impl Slot {
    pub fn try_variable(&self) -> Option<Symbol> {
        if let Slot::Variable(sym) = self {
            Some(*sym)
        } else {
            None
        }
    }
}

impl Atom {
    pub fn determinant_variables<'a, 'b>(
        &'a self,
        db: &'b Database,
    ) -> impl Iterator<Item = (usize, Symbol)> + 'a {
        let num_determinant = db.table(self.table).num_determinant();
        let slots = &self.slots[0..num_determinant];
        slots
            .into_iter()
            .enumerate()
            .filter_map(|(idx, slot)| slot.try_variable().map(|var| (idx, var)))
    }

    pub fn dependent_variables<'a, 'b>(
        &'a self,
        db: &'b Database,
    ) -> impl Iterator<Item = (usize, Symbol)> + 'a {
        let num_determinant = db.table(self.table).num_determinant();
        let slots = &self.slots[num_determinant..];
        slots
            .into_iter()
            .enumerate()
            .filter_map(|(idx, slot)| slot.try_variable().map(|var| (idx, var)))
    }
}

#[cfg(test)]
mod tests {
    use ds::uf::UnionFind;

    use crate::database::{Database, DatabaseAuxiliaryState};
    use crate::fixpoint::FunctionLibrary;
    use crate::grammar::ProgramParser;

    use super::*;

    #[test]
    fn parse1() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let mut database = Database::new(aux_state);
        let library = FunctionLibrary::new();
        let program = "#Add(EClassId EClassId -> EClassId); Add(x y z) => Add(y x z);";
        ProgramParser::new()
            .parse(&mut interner, &mut database, &library, &program)
            .unwrap();
    }
}
