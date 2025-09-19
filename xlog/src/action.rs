use std::collections::BTreeMap;

use ds::table::Value;

use crate::database::Database;
use crate::frontend::{Action, Symbol};

pub fn execute_actions(
    db: &mut Database,
    action_substs: Vec<(&Action, Vec<BTreeMap<Symbol, Value>>)>,
) -> bool {
    let mut changed = false;

    for (action, substs) in action_substs {
        match action {
            Action::InsertPattern { atoms } => {
                for subst in substs {
                    for atom in atoms {}
                }
            }
        }
    }

    changed
}
