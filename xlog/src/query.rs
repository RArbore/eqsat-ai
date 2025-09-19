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

#[cfg(test)]
mod tests {
    use crate::frontend::{Interner, Atom};

    use super::*;

    #[test]
    fn simple_query() {
        let mut interner = Interner::new();
        let symbol1 = interner.get_or_intern("x");
        let symbol2 = interner.get_or_intern("y");
        let mut database = Database::new();
        database.register_table(symbol1, 1, 2);
        database.register_table(symbol2, 1, 1);
        let id1 = database.table_id(symbol1);
        let id2 = database.table_id(symbol2);
        let table1 = database.table_mut(id1);
        table1.insert(&[0, 1, 4]);
        table1.insert(&[1, 1, 3]);
        table1.insert(&[3, 3, 2]);
        let table2 = database.table_mut(id2);
        table2.insert(&[4, 0]);
        table2.insert(&[2, 3]);
        table2.insert(&[0, 0]);
        table2.insert(&[5, 1]);
        let query = Query {
            atoms: vec![
                Atom {
                    table: 0,
                    slots: vec![Slot::Variable(symbol1), Slot::Concrete(1), Slot::Wildcard],
                },
                Atom {
                    table: 1,
                    slots: vec![Slot::Variable(symbol2), Slot::Variable(symbol1)],
                },
            ],
        };
        let matches = dumb_product_query(&database, &query);
        assert_eq!(
            matches,
            vec![
                [(symbol1, 0), (symbol2, 4)].into_iter().collect(),
                [(symbol1, 0), (symbol2, 0)].into_iter().collect(),
                [(symbol1, 1), (symbol2, 5)].into_iter().collect(),
            ]
        );
    }
}
