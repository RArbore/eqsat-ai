use xlog::database::Database;
use xlog::frontend::Interner;

use crate::ast::{FunctionAST, ProgramAST};

struct AIContext<'a, 'b> {
    db: &'a mut Database<'b>,
    interner: &'a mut Interner,
}

pub fn abstract_interpret(program: &ProgramAST, db: &mut Database, interner: &mut Interner) {
    for func in &program.funcs {
        let mut state = AIContext { db, interner };
        state.ai_func(func);
    }
}

impl<'a, 'b> AIContext<'a, 'b> {
    fn ai_func(&mut self, func: &FunctionAST) {

    }
}
