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
        let mut database = Database::new();
        let aux_state = DatabaseAuxiliaryState { uf: &uf };
        let program = "#Edge(Int Int ->); #Path(Int Int ->); #Success(-> Int); Edge(a b) => Path(a b); Path(a b) Edge(b c) => Path(a c); => Edge(0 1); => Edge(0 2); => Edge(0 3); => Edge(2 4); => Edge(4 3); => Edge(4 5); => Edge(3 0); Path(3 5) => Success(1);";
        let program = ProgramParser::new()
            .parse(&mut interner, &mut database, &aux_state, &program)
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
}
