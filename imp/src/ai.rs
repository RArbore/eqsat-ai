use xlog::database::Database;
use xlog::fixpoint::FunctionLibrary;
use xlog::frontend::{Interner, Rule, Schema, SchemaColumn};
use xlog::grammar::ProgramParser;

use crate::ast::{FunctionAST, Location, ProgramAST, StatementAST};
use crate::lattice::{MeetSemilattice, Reachability};

struct AIContext<'a, 'b> {
    db: &'a mut Database<'b>,
    interner: &'a mut Interner,
    library: &'a mut FunctionLibrary,

    rules: &'a mut Vec<Rule>,
}

pub fn abstract_interpret(
    program: &ProgramAST,
    db: &mut Database,
    interner: &mut Interner,
) -> Vec<Rule> {
    let mut rules = vec![];
    let mut library = FunctionLibrary::new();
    db.register_custom_table(
        interner.get_or_intern("Reach"),
        Schema {
            determinant: vec![SchemaColumn::Int],
            dependent: vec![SchemaColumn::CustomLattice],
        },
        Box::new(|lhs, rhs, dst| {
            dst[1] = Reachability::from(lhs[1])
                .meet(&Reachability::from(rhs[1]))
                .into()
        }),
        Box::new(|_, _| {}),
    );

    for func in &program.funcs {
        let mut state = AIContext {
            db,
            interner,
            library: &mut library,

            rules: &mut rules,
        };
        state.ai_func(func);
    }

    rules
}

impl<'a, 'b> AIContext<'a, 'b> {
    fn add_rule(&mut self, rule: &str) {
        self.rules.extend(
            ProgramParser::new()
                .parse(self.interner, self.db, self.library, rule)
                .unwrap(),
        );
    }

    fn ai_func(&mut self, func: &FunctionAST) {
        self.add_rule(&format!("=> Reach({}, 1);", func.location));
        let last_loc = self.ai_stmt(vec![func.location], &func.body);
        assert!(last_loc.is_empty());
    }

    fn ai_stmt(&mut self, prior_locs: Vec<Location>, stmt: &StatementAST) -> Vec<Location> {
        for loc in prior_locs {
            self.add_rule(&format!("Reach({}, 1) => Reach({}, 1);", loc, stmt.loc()));
        }

        use StatementAST::*;
        match stmt {
            Block(loc, stmts) => {
                let mut locs = vec![*loc];
                for stmt in stmts {
                    locs = self.ai_stmt(locs, stmt);
                }
                locs
            }
            Assign(loc, _, _) => vec![*loc],
            IfElse(loc, _, true_stmt, false_stmt) => {
                let mut locs = self.ai_stmt(vec![*loc], true_stmt);
                if let Some(false_stmt) = false_stmt {
                    locs.extend(self.ai_stmt(vec![*loc], false_stmt));
                } else {
                    locs.push(*loc);
                }
                locs
            }
            While(loc, _, stmt) => {
                let body_locs = self.ai_stmt(vec![*loc], stmt);
                for body_loc in body_locs {
                    self.add_rule(&format!("Reach({}, 1) => Reach({}, 1);", body_loc, *loc));
                }
                vec![*loc]
            }
            Return(_, _) => vec![],
        }
    }
}
