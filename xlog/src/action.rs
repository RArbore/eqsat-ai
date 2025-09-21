use std::collections::BTreeMap;

use ds::table::Value;

use crate::database::Database;
use crate::frontend::{Action, Atom, SchemaColumn, Symbol};

pub fn execute_actions(
    db: &mut Database,
    action_substs: Vec<(&Action, Vec<BTreeMap<Symbol, Value>>)>,
) -> bool {
    let mut changed = false;

    for (action, substs) in action_substs {
        for mut subst in substs {
            let mut action = action;
            loop {
                match action {
                    Action::InsertPattern { atoms } => {
                        chase(db, &mut subst, atoms);
                        for atom in atoms {
                            changed = db.insert_atom_with_subst(atom, &subst) || changed;
                        }
                        break;
                    }
                    Action::ComputeFunc { func, next } => {
                        if !func(&mut subst) {
                            break;
                        }
                        action = &next;
                    }
                }
            }
        }
    }

    changed
}

fn chase(db: &mut Database, subst: &mut BTreeMap<Symbol, Value>, atoms: &Vec<Atom>) {
    loop {
        let mut changed = false;

        for atom in atoms {
            if atom
                .determinant_variables(db)
                .all(|(_, var)| subst.contains_key(&var))
                && let Some(in_dependent) = db.get_with_subst(atom, subst)
            {
                for (idx, var) in atom.dependent_variables(db) {
                    if !subst.contains_key(&var) {
                        changed = true;
                        subst.insert(var, in_dependent[idx]);
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }

    for atom in atoms {
        let schema = db.schema(atom.table);
        //for (_, var) in atom.determinant_variables(db) {
        //    assert!(subst.contains_key(&var));
        //}
        for (idx, var) in atom.dependent_variables(db) {
            if !subst.contains_key(&var) {
                let val = match schema.dependent[idx] {
                    SchemaColumn::EClassId => db.aux_state().uf.makeset().into(),
                    _ => panic!(),
                };
                subst.insert(var, val);
            }
        }
    }
}
