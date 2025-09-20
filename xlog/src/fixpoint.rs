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
    use ds::uf::UnionFind;

    use crate::database::{Database, DatabaseAuxiliaryState};
    use crate::frontend::Interner;
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
}
