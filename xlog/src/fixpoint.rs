use crate::action::execute_actions;
use crate::database::Database;
use crate::frontend::Rule;
use crate::query::dumb_product_query;

pub fn fixpoint(db: &mut Database, program: &Vec<Rule>) {
    loop {
        let mut matches = vec![];
        for rule in program {
            let matched = dumb_product_query(db, &rule.query);
            matches.push((&rule.action, matched));
        }

        let mut changed = execute_actions(db, matches);
        changed = db.repair() || changed;

        if !changed {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use core::cmp::max;
    use std::collections::BTreeMap;

    use ds::table::Value;
    use ds::uf::UnionFind;

    use crate::database::{Database, DatabaseAuxiliaryState};
    use crate::frontend::{Action, Atom, Interner, Query, Rule, Slot, Symbol};
    use crate::grammar::ProgramParser;

    use super::*;

    #[test]
    fn simple_graph() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let mut database = Database::new(aux_state);
        let program = "#Edge(Int Int ->); #Path(Int Int ->); #Success(-> Int); Edge(a b) => Path(a b); Path(a b) Edge(b c) => Path(a c); => Edge(0 1); => Edge(0 2); => Edge(0 3); => Edge(2 4); => Edge(4 3); => Edge(4 5); => Edge(3 0); Path(3 5) => Success(1);";
        let program = ProgramParser::new()
            .parse(&mut interner, &mut database, &program)
            .unwrap();
        fixpoint(&mut database, &program);
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Edge")))
                .rows(false)
                .count(),
            7
        );
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Path")))
                .rows(false)
                .count(),
            24
        );
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Success")))
                .rows(false)
                .count(),
            1
        );
    }

    #[test]
    fn simple_chase() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let mut database = Database::new(aux_state);
        let program = "#Constant(Int -> EClassId); #Add(EClassId EClassId -> EClassId); Add(x y z) => Add(y x z); => Constant(1 a); => Constant(2 a); Constant(_ a) Constant(_ b) => Add(a b z);";
        let program = ProgramParser::new()
            .parse(&mut interner, &mut database, &program)
            .unwrap();
        fixpoint(&mut database, &program);
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Constant")))
                .rows(false)
                .count(),
            2
        );
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Add")))
                .rows(false)
                .count(),
            4
        );
    }

    #[test]
    fn simple_rewrite() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let mut database = Database::new(aux_state);
        let program = "#Constant(Int -> EClassId); #Add(EClassId EClassId -> EClassId); Add(x y z) => Add(y x z); Add(a b ab) Add(ab c total) => Add(a bc total) Add(b c bc); => Constant(1 one); => Constant(2 two); => Constant(3 three); Constant(1 one) Constant(2 two) Constant(3 three) => Add(one two one_plus_two) Add(one_plus_two three one_plus_two_plus_three);";
        let program = ProgramParser::new()
            .parse(&mut interner, &mut database, &program)
            .unwrap();
        fixpoint(&mut database, &program);
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Constant")))
                .rows(false)
                .count(),
            3
        );
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Add")))
                .rows(false)
                .count(),
            12
        );
    }

    #[test]
    fn computed_action() {
        let uf = UnionFind::new();
        let mut interner = Interner::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let mut database = Database::new(aux_state);
        let program = "#Constant(Int -> EClassId); #Max(EClassId EClassId -> EClassId); => Constant(77 first); => Constant(42 second); Constant(_ first) Constant(_ second) => Max(first second first_plus_second);";
        let mut program = ProgramParser::new()
            .parse(&mut interner, &mut database, &program)
            .unwrap();

        let constant_id = database.table_id(interner.get_or_intern("Constant"));
        let max_id = database.table_id(interner.get_or_intern("Max"));
        let lhs_sym = interner.get_or_intern("lhs");
        let rhs_sym = interner.get_or_intern("rhs");
        let lhs_cons_sym = interner.get_or_intern("lhs_cons");
        let rhs_cons_sym = interner.get_or_intern("rhs_cons");
        let max_sym = interner.get_or_intern("max");
        let lhs_plus_rhs_sym = interner.get_or_intern("lhs_plus_rhs");

        program.push(Rule {
            query: Query {
                atoms: vec![
                    Atom {
                        table: constant_id,
                        slots: vec![Slot::Variable(lhs_cons_sym), Slot::Variable(lhs_sym)],
                    },
                    Atom {
                        table: constant_id,
                        slots: vec![Slot::Variable(rhs_cons_sym), Slot::Variable(rhs_sym)],
                    },
                    Atom {
                        table: max_id,
                        slots: vec![
                            Slot::Variable(lhs_sym),
                            Slot::Variable(rhs_sym),
                            Slot::Variable(max_sym),
                        ],
                    },
                ],
            },
            action: Action::ComputeFunc {
                func: Box::new(move |syms: &mut BTreeMap<Symbol, Value>| -> bool {
                    let lhs = syms[&lhs_cons_sym];
                    let rhs = syms[&rhs_cons_sym];
                    syms.insert(lhs_plus_rhs_sym, max(lhs, rhs));
                    true
                }),
                next: Box::new(Action::InsertPattern {
                    atoms: vec![Atom {
                        table: constant_id,
                        slots: vec![Slot::Variable(lhs_plus_rhs_sym), Slot::Variable(max_sym)],
                    }],
                }),
            },
        });

        fixpoint(&mut database, &program);

        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Constant")))
                .rows(false)
                .count(),
            2
        );
        assert_eq!(
            database
                .table(database.table_id(interner.get_or_intern("Max")))
                .rows(false)
                .count(),
            4
        );
        assert!(
            database
                .table(database.table_id(interner.get_or_intern("Constant")))
                .rows(false)
                .any(|(row, _)| row[0] == 77)
        );
    }
}
