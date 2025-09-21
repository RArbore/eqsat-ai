use std::collections::BTreeMap;

use xlog::database::Database;
use xlog::frontend::{Interner, Symbol};

use crate::ast::{FunctionAST, ProgramAST, StatementAST};

struct AIContext<'a, 'b> {
    db: &'a mut Database<'b>,
    interner: &'a mut Interner,
    snapshot: AISnapshot,
}

#[derive(Clone, Debug)]
struct AISnapshot {
    last_location: u32,
    last_def: BTreeMap<Symbol, u32>,
}

pub fn abstract_interpret(program: &ProgramAST, db: &mut Database, interner: &mut Interner) {
    for func in &program.funcs {
        let mut state = AIContext {
            db,
            interner,
            snapshot: AISnapshot {
                last_location: 0,
                last_def: BTreeMap::new(),
            },
        };
        state.ai_func(func);
    }
}

impl<'a, 'b> AIContext<'a, 'b> {
    fn ai_func(&mut self, func: &FunctionAST) {
        self.snapshot.last_location = func.location;
        for param in &func.params {
            self.snapshot.last_def.insert(*param, func.location);
        }
        self.ai_stmt(&func.body);
    }

    fn ai_stmt(&mut self, stmt: &StatementAST) {}
}
