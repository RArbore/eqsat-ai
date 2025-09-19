use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU16;

use ds::table::Value;

use crate::database::TableId;

pub(crate) type Symbol = SymbolU16;
pub(crate) type Interner = StringInterner<StringBackend<Symbol>>;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Slot {
    Wildcard,
    Variable(Symbol),
    Concrete(Value),
}

#[derive(Clone, Debug)]
pub(crate) struct Atom {
    pub(crate) table: TableId,
    pub(crate) slots: Vec<Slot>,
}

#[derive(Clone, Debug)]
pub(crate) struct Query {
    pub(crate) atoms: Vec<Atom>,
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    InsertPattern { atoms: Vec<Atom> },
}

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub(crate) query: Query,
    pub(crate) action: Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::database::Database;
    use crate::grammar::ProgramParser;

    #[test]
    fn parse1() {
        let mut interner = Interner::new();
        let mut database = Database::new();
        let program = ".decl(Add 2 1); Add(x y z) => Add(y x z);";
        assert_eq!(
            format!(
                "{:?}",
                ProgramParser::new()
                    .parse(&mut interner, &mut database, &program)
                    .unwrap()
            ),
            "[Rule { query: Query { atoms: [Atom { table: 0, slots: [Variable(SymbolU16 { value: 2 }), Variable(SymbolU16 { value: 3 }), Variable(SymbolU16 { value: 4 })] }] }, action: InsertPattern { atoms: [Atom { table: 0, slots: [Variable(SymbolU16 { value: 3 }), Variable(SymbolU16 { value: 2 }), Variable(SymbolU16 { value: 4 })] }] } }]"
        );
    }
}
