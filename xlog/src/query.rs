use core::iter::zip;
use std::collections::BTreeMap;

use ds::table::Value;

use crate::database::{Database};
use crate::frontend::{Slot, Query, Symbol};

pub fn dumb_product_query(db: &Database, query: &Query) -> Vec<BTreeMap<Symbol, Value>> {
    let mut subquery = query.clone();
    let Some(atom) = subquery.atoms.pop() else {
        return vec![BTreeMap::new()];
    };

    let submatches = dumb_product_query(db, &subquery);
    let table = db.table(atom.table);
    let mut matches = vec![];
    for m in submatches {
        for row in table.rows(false) {
            assert_eq!(row.0.len(), atom.slots.len());
            let mut new_match = m.clone();
            let mut matched = true;
            for (value, slot) in zip(row.0.iter(), atom.slots.iter()) {
                use Slot::*;
                match slot {
                    Wildcard => {}
                    Variable(sym) => {
                        if let Some(old_value) = new_match.get(sym) {
                            if old_value != value {
                                matched = false;
                                break;
                            }
                        } else {
                            new_match.insert(*sym, *value);
                        }
                    }
                    Concrete(concrete) => {
                        if concrete != value {
                            matched = false;
                            break;
                        }
                    }
                }
            }
            if matched {
                matches.push(new_match);
            }
        }
    }

    matches
}
