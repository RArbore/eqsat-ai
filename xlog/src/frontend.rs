use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

use ds::table::Value;

use crate::database::TableId;

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

#[derive(Clone, Debug)]
pub enum Action {
    InsertPattern { atoms: Vec<Atom> },
}

#[derive(Clone, Debug)]
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
    Int,
}

#[cfg(test)]
mod tests {
    use ds::uf::UnionFind;

    use crate::database::{Database, DatabaseAuxiliaryState};
    use crate::grammar::ProgramParser;

    use super::*;

    #[test]
    fn parse1() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let mut database = Database::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let program = "#Add(EClassId EClassId -> EClassId); Add(x y z) => Add(y x z);";
        assert_eq!(
            format!(
                "{:?}",
                ProgramParser::new()
                    .parse(&mut interner, &mut database, &aux_state, &program)
                    .unwrap()
            ),
            "[Rule { query: Query { atoms: [Atom { table: 0, slots: [Variable(SymbolU16 { value: 2 }), Variable(SymbolU16 { value: 3 }), Variable(SymbolU16 { value: 4 })] }] }, action: InsertPattern { atoms: [Atom { table: 0, slots: [Variable(SymbolU16 { value: 3 }), Variable(SymbolU16 { value: 2 }), Variable(SymbolU16 { value: 4 })] }] } }]"
        );
    }
}
