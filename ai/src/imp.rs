use std::collections::HashMap;

use imp::ast::Symbol;
use imp::ast::{BlockAST, ExpressionAST, FunctionAST, StatementAST};

use crate::domain::AbstractDomain;

pub fn ai_func<AD>(
    mut ad: AD,
    function: &FunctionAST,
    param_abstractions: &HashMap<Symbol, AD::Value>,
) where
    AD: AbstractDomain<Variable = Symbol, Expression = ExpressionAST>,
{
    let mut unique_id = 0;
    for param in &function.params {
        if let Some(abstraction) = param_abstractions.get(param) {
            ad.assign(*param, abstraction.clone());
        } else {
            ad.assign(*param, ad.bottom());
        }
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
            let unique_id_fix = *unique_id;
            let cond = ad.forward_transfer(expr);
            let (true_ad, false_ad) = ad.branch(cond);
            let true_ad = true_ad.and_then(|true_ad| ai_block(true_ad, true_block, unique_id));
            let false_ad = false_ad.and_then(|false_ad| {
                if let Some(false_block) = false_block {
                    ai_block(false_ad, false_block, unique_id)
                } else {
                    Some(false_ad)
                }
            });

            match (true_ad, false_ad) {
                (Some(true_ad), Some(false_ad)) => Some(true_ad.join(&false_ad, unique_id_fix)),
                (Some(ad), None) | (None, Some(ad)) => Some(ad),
                (None, None) => None,
            }
        }
        While(expr, block) => {
            let unique_id_fix = *unique_id;
            let init = ad.clone();
            loop {
                let cond = ad.forward_transfer(expr);
                let (cont, exit) = ad.clone().branch(cond);
                let Some(bottom) = cont.and_then(|cont| ai_block(cont, block, unique_id)) else {
                    break exit;
                };
                let widened = init.widen(&bottom, unique_id_fix);
                if ad == widened {
                    break exit;
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
    use std::collections::{BTreeMap, HashSet};

    use ds::egraph::EGraph;
    use ds::uf::ClassId;
    use imp::ast::Interner;
    use imp::grammar::ProgramParser;

    use crate::concrete::Concrete;
    use crate::domain::{Lattice, LatticeDomain};
    use crate::essa::{ESSADomain, Term};
    use crate::interval::Interval;

    #[test]
    fn abstract_interpret1() {
        let mut interner = Interner::new();
        let program =
            "fn basic(x, y) { if 0 { return (x < y) * 5; } else { return (y > x) - 3; } }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<Symbol, Interval, ExpressionAST>::new(&finished);
        ai_func(ad, &program.funcs[0], &HashMap::new());
        let joined = finished
            .into_inner()
            .values()
            .map(|x| *x)
            .reduce(|a, b| a.join(&b))
            .unwrap();
        assert_eq!(joined, Interval { low: -3, high: -2 });
    }

    #[test]
    fn abstract_interpret2() {
        let mut interner = Interner::new();
        let program = "fn basic() { x = 10; while x { x = x / 2; } return x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<Symbol, Interval, ExpressionAST>::new(&finished);
        ai_func(ad, &program.funcs[0], &HashMap::new());
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
        let program =
            "fn basic(x, y, z) { if x > y { z = x + y; } else { y = z - x; } return z + y + x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let num_params = Cell::new(0);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis, ());
        ai_func(ad, &program.funcs[0], &HashMap::new());
        graph.borrow_mut().full_repair();
    }

    #[test]
    fn abstract_interpret4() {
        let mut interner = Interner::new();
        let program = "fn basic(x) { y = x; while x { x = x - 1; y = y - 1; } return y + x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let num_params = Cell::new(0);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis, ());
        ai_func(ad, &program.funcs[0], &HashMap::new());
        graph.borrow_mut().full_repair();
    }

    #[test]
    fn abstract_interpret5() {
        let mut interner = Interner::new();
        let program = "fn basic(x, y) { while x < 100 { x = x + 7; } if y { x = x + 17; } else { x = 120; } return x; }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<ClassId, Concrete, Term>::new(&finished);
        let num_params = Cell::new(1);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis, ad);
        let mut param_abstractions = HashMap::new();
        let param = graph.borrow_mut().makeset();
        graph.borrow_mut().insert(&Term::Parameter(0, param));
        param_abstractions.insert(interner.get_or_intern("x"), (param, Concrete::Value(5)));
        ai_func(ad, &program.funcs[0], &param_abstractions);
        assert_eq!(
            finished.into_inner().into_iter().next().unwrap().1,
            Concrete::Value(120),
        );
    }

    #[test]
    fn abstract_interpret6() {
        let mut interner = Interner::new();
        let program = "fn basic(x) { if x { return 10 + 5; } else { return 7 + 2; } }";
        let program = ProgramParser::new().parse(&mut interner, &program).unwrap();
        let finished = RefCell::new(BTreeMap::new());
        let ad = LatticeDomain::<ClassId, Concrete, Term>::new(&finished);
        let num_params = Cell::new(0);
        let graph = RefCell::new(EGraph::new());
        let static_phis = RefCell::new(BTreeMap::new());
        let ad = ESSADomain::new(&num_params, &graph, &static_phis, ad);
        ai_func(ad, &program.funcs[0], &HashMap::new());
        let finished: HashSet<_> = finished
            .into_inner()
            .into_iter()
            .map(|(_, val)| val)
            .collect();
        assert_eq!(
            finished,
            HashSet::from_iter(vec![Concrete::Value(15), Concrete::Value(9)].into_iter())
        );
    }
}
