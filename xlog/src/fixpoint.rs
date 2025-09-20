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
