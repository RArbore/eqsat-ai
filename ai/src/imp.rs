use imp::ast::Symbol;
use imp::ast::{BlockAST, ExpressionAST, FunctionAST, StatementAST};

use crate::domain::AbstractDomain;

pub fn ai_func<AD>(mut ad: AD, function: &FunctionAST)
where
    AD: AbstractDomain<Variable = Symbol, Expression = ExpressionAST>,
{
    let mut unique_id = 0;
    for param in &function.params {
        ad.assign(*param, ad.bottom());
    }
    ai_block(ad, &function.block, &mut unique_id);
}

pub fn ai_block<AD>(mut ad: AD, block: &BlockAST, unique_id: &mut usize) -> Option<AD>
where
    AD: AbstractDomain<Variable = Symbol, Expression = ExpressionAST>,
{
    for stmt in &block.stmts {
        ad = ai_stmt(ad, stmt, unique_id)?;
    }
    Some(ad)
}

pub fn ai_stmt<AD>(mut ad: AD, stmt: &StatementAST, unique_id: &mut usize) -> Option<AD>
where
    AD: AbstractDomain<Variable = Symbol, Expression = ExpressionAST>,
{
    *unique_id = *unique_id + 1;

    use StatementAST::*;
    match stmt {
        Block(block) => ai_block(ad, block, unique_id),
        Assign(symbol, expr) => {
            let val = ad.forward_transfer(expr);
            ad.assign(*symbol, val);
            Some(ad)
        }
        IfElse(expr, true_block, false_block) => {
            let _cond = ad.forward_transfer(expr);
            let (true_ad, false_ad) = ad.branch();
            let true_ad = ai_block(true_ad, true_block, unique_id);
            let false_ad = if let Some(false_block) = false_block {
                ai_block(false_ad, false_block, unique_id)
            } else {
                Some(false_ad)
            };

            match (true_ad, false_ad) {
                (Some(true_ad), Some(false_ad)) => Some(true_ad.join(&false_ad)),
                (Some(ad), None) | (None, Some(ad)) => Some(ad),
                (None, None) => None,
            }
        }
        While(expr, block) => {
            let unique_id_fix = *unique_id;
            let init = ad.clone();
            loop {
                let _cond = ad.forward_transfer(expr);
                let (cont, exit) = ad.clone().branch();
                let Some(bottom) = ai_block(cont, block, unique_id) else {
                    break Some(exit);
                };
                let widened = init.widen(&bottom, unique_id_fix);
                if ad == widened {
                    break Some(exit);
                } else {
                    ad = widened;
                }
            }
        }
        Return(expr) => {
            let val = ad.forward_transfer(expr);
            ad.finish(val, *unique_id);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::cell::{Cell, RefCell};
    use std::collections::BTreeMap;

    use ds::egraph::EGraph;
    use imp::ast::Interner;
    use imp::grammar::ProgramParser;

    use crate::domain::{Lattice, LatticeDomain};
    use crate::interval::Interval;
    use crate::ssa::ESSADomain;

    #[test]
    fn abstract_interpret1() {
        let mut interner = Interner::new();
        let program =
            "fn basic(x, y) { if y { return (x < y) * 5; } else { return (y > x) - 3; } }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<Symbol, Interval>::new(&finished);
        ai_func(ad, &program.funcs[0]);
        let joined = finished
            .into_inner()
            .values()
            .map(|x| *x)
            .reduce(|a, b| a.join(&b))
            .unwrap();
        assert_eq!(joined, Interval { low: -3, high: 5 });
    }

    #[test]
    fn abstract_interpret2() {
        let mut interner = Interner::new();
        let program = "fn basic() { x = 10; while x { x = x / 2; } return x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<Symbol, Interval>::new(&finished);
        ai_func(ad, &program.funcs[0]);
        assert_eq!(
            finished.into_inner().into_iter().next().unwrap().1,
            Interval {
                low: i32::MIN,
                high: 10
            }
        );
    }

    #[test]
    fn abstract_interpret3() {
        let mut interner = Interner::new();
        let program = "fn basic(x, y, z) { if x > y { z = x + y; } else { y = z - x; } return z + y + x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let num_params = Cell::new(0);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis);
        ai_func(ad, &program.funcs[0]);
        graph.borrow_mut().rebuild();
    }

    #[test]
    fn abstract_interpret4() {
        let mut interner = Interner::new();
        let program = "fn basic(x) { while x { x = x - 1; } return x + 1; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let num_params = Cell::new(0);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis);
        ai_func(ad, &program.funcs[0]);
        graph.borrow_mut().rebuild();
    }
}
