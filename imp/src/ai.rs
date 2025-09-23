use string_interner::symbol::Symbol as _;

use ds::table::Value;

use xlog::database::Database;
use xlog::fixpoint::FunctionLibrary;
use xlog::frontend::{Interner, Rule, Schema, SchemaColumn, Symbol};
use xlog::grammar::ProgramParser;

use crate::ast::{ExpressionAST, FunctionAST, Location, ProgramAST, StatementAST};
use crate::lattice::{Constant, MeetSemilattice, Reachability};

struct AIContext<'a, 'b> {
    db: &'a mut Database<'b>,
    interner: &'a mut Interner,
    library: &'a mut FunctionLibrary,
    func: &'a FunctionAST,
    rules: &'a mut Vec<Rule>,

    vars: Vec<Symbol>,
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
        Box::new(|row, dst| dst.copy_from_slice(row)),
    );

    db.register_custom_table(
        interner.get_or_intern("Const"),
        Schema {
            determinant: vec![SchemaColumn::Symbol, SchemaColumn::Int],
            dependent: vec![SchemaColumn::CustomLattice, SchemaColumn::CustomLattice],
        },
        Box::new(|lhs, rhs, dst| {
            let lhs: [Value; 2] = lhs[2..4].try_into().unwrap();
            let rhs: [Value; 2] = rhs[2..4].try_into().unwrap();
            let arr: [Value; 2] = Constant::from(lhs).meet(&Constant::from(rhs)).into();
            dst[2..4].copy_from_slice(&arr);
        }),
        Box::new(|row, dst| dst.copy_from_slice(row)),
    );

    for func in &program.funcs {
        let mut state = AIContext::new(db, interner, &mut library, func, &mut rules);
        state.ai_func();
    }

    rules
}

impl<'a, 'b> AIContext<'a, 'b> {
    fn new(
        db: &'a mut Database<'b>,
        interner: &'a mut Interner,
        library: &'a mut FunctionLibrary,
        func: &'a FunctionAST,
        rules: &'a mut Vec<Rule>,
    ) -> Self {
        AIContext {
            db,
            interner,
            library,
            func,
            rules,

            vars: collect_vars(func),
        }
    }

    fn add_rule(&mut self, rule: &str) {
        self.rules.extend(
            ProgramParser::new()
                .parse(self.interner, self.db, self.library, rule)
                .expect(&format!("couldn't parse rule \"{}\"", rule)),
        );
    }

    fn ai_func(&mut self) {
        self.add_rule(&format!("=> Reach({} 0);", self.func.location));
        self.add_rule(&format!("=> Reach({} 1);", self.func.location));

        for var in self.vars.clone() {
            self.add_rule(&format!(
                "=> Const({} {} 1 0);",
                var.to_usize(),
                self.func.location
            ));
        }

        for param in self.func.params.clone() {
            self.add_rule(&format!(
                "=> Const({} {} 2 0);",
                param.to_usize(),
                self.func.location
            ));
        }

        let last_loc = self.ai_stmt(vec![self.func.location], &self.func.body);
        assert!(last_loc.is_empty());
    }

    fn ai_stmt(&mut self, prior_locs: Vec<Location>, stmt: &StatementAST) -> Vec<Location> {
        use StatementAST::*;
        let assigned_var = if let Assign(_, var, _) = stmt {
            Some(*var)
        } else {
            None
        };

        self.add_rule(&format!("=> Reach({} 0);", stmt.loc()));
        for loc in prior_locs {
            self.add_rule(&format!("Reach({} 1) => Reach({} 1);", loc, stmt.loc()));
            for var in self.vars.clone() {
                if Some(var) != assigned_var {
                    self.add_rule(&format!("Reach({} 1) Const({} {} c1 c2) => Const({} {} c1 c2);", loc, var.to_usize(), loc, var.to_usize(), stmt.loc()));
                }
            }
        }

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

fn collect_vars(func: &FunctionAST) -> Vec<Symbol> {
    let mut stmts = vec![&func.body];
    let mut exprs = vec![];
    let mut vars = func.params.clone();

    while let Some(stmt) = stmts.pop() {
        use StatementAST::*;
        match stmt {
            Block(_, body) => stmts.extend(body),
            Assign(_, var, expr) => {
                vars.push(*var);
                exprs.push(expr);
            }
            IfElse(_, cond, true_stmt, false_stmt) => {
                exprs.push(cond);
                stmts.push(true_stmt);
                if let Some(false_stmt) = false_stmt {
                    stmts.push(false_stmt);
                }
            }
            While(_, cond, body) => {
                exprs.push(cond);
                stmts.push(body);
            }
            Return(_, expr) => exprs.push(expr),
        }
    }

    while let Some(expr) = exprs.pop() {
        use ExpressionAST::*;
        match expr {
            NumberLiteral(_) => {}
            Variable(var) => vars.push(*var),
            Call(_, _) => todo!(),
            Add(lhs, rhs)
            | Subtract(lhs, rhs)
            | Multiply(lhs, rhs)
            | Divide(lhs, rhs)
            | Modulo(lhs, rhs)
            | EqualsEquals(lhs, rhs)
            | NotEquals(lhs, rhs)
            | Less(lhs, rhs)
            | LessEquals(lhs, rhs)
            | Greater(lhs, rhs)
            | GreaterEquals(lhs, rhs) => {
                exprs.push(lhs);
                exprs.push(rhs);
            }
        }
    }

    vars
}
