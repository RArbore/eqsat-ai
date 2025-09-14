use core::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use bitvec::bitarr;
use bitvec::prelude::*;

use ds::egraph::{EGraph, ENode, Signature};
use ds::uf::ClassId;
use imp::ast::{ExpressionAST, Symbol};

use crate::domain::AbstractDomain;
use crate::intersect_btree_maps;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BlockId(u32);

impl From<u32> for BlockId {
    fn from(value: u32) -> Self {
        BlockId(value)
    }
}

impl From<BlockId> for u32 {
    fn from(value: BlockId) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Term {
    Constant(i32, ClassId),
    Parameter(u32, ClassId),

    Phi(BlockId, ClassId, ClassId, ClassId),

    Add(ClassId, ClassId, ClassId),
    Subtract(ClassId, ClassId, ClassId),
    Multiply(ClassId, ClassId, ClassId),
    Divide(ClassId, ClassId, ClassId),
    Modulo(ClassId, ClassId, ClassId),
    EqualsEquals(ClassId, ClassId, ClassId),
    NotEquals(ClassId, ClassId, ClassId),
    Less(ClassId, ClassId, ClassId),
    LessEquals(ClassId, ClassId, ClassId),
    Greater(ClassId, ClassId, ClassId),
    GreaterEquals(ClassId, ClassId, ClassId),
}

impl ENode for Term {
    fn signature(&self) -> Signature {
        use Term::*;
        match self {
            Constant(..) => Signature {
                class_id_mask: bitarr![0, 1],
                num_det_cols: 1,
                num_dep_cols: 1,
                symbol_id: 0,
            },
            Parameter(..) => Signature {
                class_id_mask: bitarr![0, 1],
                num_det_cols: 1,
                num_dep_cols: 1,
                symbol_id: 1,
            },
            Phi(..) => Signature {
                class_id_mask: bitarr![0, 1, 1, 1],
                num_det_cols: 3,
                num_dep_cols: 1,
                symbol_id: 2,
            },
            Add(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 3,
            },
            Subtract(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 4,
            },
            Multiply(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 5,
            },
            Divide(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 6,
            },
            Modulo(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 7,
            },
            EqualsEquals(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 8,
            },
            NotEquals(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 9,
            },
            Less(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 10,
            },
            LessEquals(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 11,
            },
            Greater(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 12,
            },
            GreaterEquals(..) => Signature {
                class_id_mask: bitarr![1, 1, 1],
                num_det_cols: 2,
                num_dep_cols: 1,
                symbol_id: 13,
            },
        }
    }

    fn encode_to_row(&self, det: &mut [u32], dep: &mut [u32]) {
        use Term::*;
        match self {
            Constant(cons, root) => {
                det[0] = cons.cast_unsigned();
                dep[0] = u32::from(*root);
            }
            Parameter(idx, root) => {
                det[0] = *idx;
                dep[0] = u32::from(*root);
            }
            Phi(block, lhs, rhs, root) => {
                det[0] = u32::from(*block);
                det[1] = u32::from(*lhs);
                det[2] = u32::from(*rhs);
                dep[0] = u32::from(*root);
            }
            Add(lhs, rhs, root)
            | Subtract(lhs, rhs, root)
            | Multiply(lhs, rhs, root)
            | Divide(lhs, rhs, root)
            | Modulo(lhs, rhs, root)
            | EqualsEquals(lhs, rhs, root)
            | NotEquals(lhs, rhs, root)
            | Less(lhs, rhs, root)
            | LessEquals(lhs, rhs, root)
            | Greater(lhs, rhs, root)
            | GreaterEquals(lhs, rhs, root) => {
                det[0] = u32::from(*lhs);
                det[1] = u32::from(*rhs);
                dep[0] = u32::from(*root);
            }
        }
    }

    fn decode_from_row(det: &[u32], dep: &[u32], sig: Signature) -> Self {
        use Term::*;
        match sig.symbol_id {
            0 => Constant(det[0].cast_signed(), ClassId::from(dep[0])),
            1 => Parameter(det[0], ClassId::from(dep[0])),
            2 => Phi(
                BlockId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(det[2]),
                ClassId::from(dep[0]),
            ),
            3 => Add(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            4 => Subtract(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            5 => Multiply(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            6 => Divide(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            7 => Modulo(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            8 => EqualsEquals(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            9 => NotEquals(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            10 => Less(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            11 => LessEquals(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            12 => Greater(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            13 => GreaterEquals(
                ClassId::from(det[0]),
                ClassId::from(det[1]),
                ClassId::from(dep[0]),
            ),
            _ => todo!(),
        }
    }
}

#[derive(Clone)]
pub struct ESSADomain<'a, AD>
where
    AD: AbstractDomain<Variable = ClassId, Expression = Term>,
{
    var_to_val: BTreeMap<Symbol, ClassId>,
    num_params: &'a Cell<u32>,
    graph: &'a RefCell<EGraph<Term>>,
    static_phis: &'a RefCell<BTreeMap<usize, BTreeMap<Symbol, (ClassId, ClassId)>>>,

    ad: AD,
}

impl<'a, AD> ESSADomain<'a, AD>
where
    AD: AbstractDomain<Variable = ClassId, Expression = Term>,
{
    pub fn new(
        num_params: &'a Cell<u32>,
        graph: &'a RefCell<EGraph<Term>>,
        static_phis: &'a RefCell<BTreeMap<usize, BTreeMap<Symbol, (ClassId, ClassId)>>>,
        ad: AD,
    ) -> Self {
        Self {
            var_to_val: BTreeMap::new(),
            num_params,
            graph,
            static_phis,
            ad,
        }
    }
}

impl<'a, AD> PartialEq for ESSADomain<'a, AD>
where
    AD: AbstractDomain<Variable = ClassId, Expression = Term>,
{
    fn eq(&self, other: &Self) -> bool {
        self.var_to_val == other.var_to_val && self.ad == other.ad
    }
}

impl<'a, AD> AbstractDomain for ESSADomain<'a, AD>
where
    AD: AbstractDomain<Variable = ClassId, Expression = Term>,
{
    type Variable = Symbol;
    type Value = (ClassId, AD::Value);
    type Expression = ExpressionAST;

    fn bottom(&self) -> (ClassId, AD::Value) {
        let idx = self.num_params.get() as u32;
        self.num_params.set(idx + 1);
        let root = self.graph.borrow_mut().makeset();
        (
            self.graph.borrow_mut().insert(&Term::Parameter(idx, root)),
            self.ad.bottom(),
        )
    }

    fn forward_transfer(&mut self, expr: &ExpressionAST) -> (ClassId, AD::Value) {
        let handle_binary_op =
            |s: &mut Self, mk_term: &dyn Fn(ClassId, ClassId, ClassId) -> Term, lhs, rhs| {
                let lhs = s.forward_transfer(lhs);
                let rhs = s.forward_transfer(rhs);
                let root = s.graph.borrow_mut().makeset();
                let term = mk_term(lhs.0, rhs.0, root);
                (
                    s.graph.borrow_mut().insert(&term),
                    s.ad.forward_transfer(&term),
                )
            };
        use ExpressionAST::*;
        let (class_id, ad_value) = match expr {
            NumberLiteral(lit) => {
                let root = self.graph.borrow_mut().makeset();
                let term = Term::Constant(*lit, root);
                (
                    self.graph.borrow_mut().insert(&term),
                    self.ad.forward_transfer(&term),
                )
            }
            Variable(var) => self.lookup(*var),
            Call(_, _) => todo!(),
            Add(lhs, rhs) => {
                handle_binary_op(self, &|lhs, rhs, root| Term::Add(lhs, rhs, root), lhs, rhs)
            }
            Subtract(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::Subtract(lhs, rhs, root),
                lhs,
                rhs,
            ),
            Multiply(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::Multiply(lhs, rhs, root),
                lhs,
                rhs,
            ),
            Divide(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::Divide(lhs, rhs, root),
                lhs,
                rhs,
            ),
            Modulo(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::Modulo(lhs, rhs, root),
                lhs,
                rhs,
            ),
            EqualsEquals(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::EqualsEquals(lhs, rhs, root),
                lhs,
                rhs,
            ),
            NotEquals(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::NotEquals(lhs, rhs, root),
                lhs,
                rhs,
            ),
            Less(lhs, rhs) => {
                handle_binary_op(self, &|lhs, rhs, root| Term::Less(lhs, rhs, root), lhs, rhs)
            }
            LessEquals(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::LessEquals(lhs, rhs, root),
                lhs,
                rhs,
            ),
            Greater(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::Greater(lhs, rhs, root),
                lhs,
                rhs,
            ),
            GreaterEquals(lhs, rhs) => handle_binary_op(
                self,
                &|lhs, rhs, root| Term::GreaterEquals(lhs, rhs, root),
                lhs,
                rhs,
            ),
        };
        self.ad.assign(class_id, ad_value.clone());
        (class_id, ad_value)
    }

    fn lookup(&self, var: Symbol) -> (ClassId, AD::Value) {
        let class_id = self.var_to_val[&var];
        (class_id, self.ad.lookup(class_id))
    }

    fn assign(&mut self, var: Symbol, val: (ClassId, AD::Value)) {
        self.var_to_val.insert(var, val.0);
        self.ad.assign(val.0, val.1);
    }

    fn branch(self, cond: (ClassId, AD::Value)) -> (Option<Self>, Option<Self>) {
        let (ad_true, ad_false) = self.ad.branch(cond.1);
        let self_clone = |ad| Self {
            var_to_val: self.var_to_val.clone(),
            num_params: self.num_params,
            graph: self.graph,
            static_phis: self.static_phis,
            ad,
        };
        (ad_true.map(self_clone), ad_false.map(self_clone))
    }

    fn finish(self, returned: (ClassId, AD::Value), unique_id: usize) {
        self.ad.finish(returned.1, unique_id);
    }

    fn join(&self, other: &Self, unique_id: usize) -> Self {
        let mut self_ad = self.ad.clone();
        let mut other_ad = other.ad.clone();
        let mut var_to_val = BTreeMap::new();
        for (var, self_val, other_val) in intersect_btree_maps(&self.var_to_val, &other.var_to_val)
        {
            let root = if *self_val == *other_val {
                *self_val
            } else {
                let root = self.graph.borrow_mut().makeset();
                self_ad.assign(root, self_ad.lookup(*self_val));
                other_ad.assign(root, other_ad.lookup(*other_val));
                self.graph.borrow_mut().insert(&Term::Phi(
                    BlockId(unique_id as u32),
                    *self_val,
                    *other_val,
                    root,
                ))
            };
            var_to_val.insert(*var, root);
        }
        Self {
            var_to_val,
            num_params: self.num_params,
            graph: self.graph,
            static_phis: self.static_phis,
            ad: self_ad.join(&other_ad, unique_id),
        }
    }

    fn widen(&self, other: &Self, unique_id: usize) -> Self {
        let mut self_ad = self.ad.clone();
        let mut other_ad = other.ad.clone();
        let mut static_phis_borrow = self.static_phis.borrow_mut();
        let static_phis = static_phis_borrow.entry(unique_id).or_default();
        let mut var_to_val = BTreeMap::new();
        let mut new_static_phi = false;

        for (var, self_val, other_val) in intersect_btree_maps(&self.var_to_val, &other.var_to_val)
        {
            let mut make_phi = || {
                let phi = self.graph.borrow_mut().makeset();
                self_ad.assign(phi, self_ad.lookup(*self_val));
                other_ad.assign(phi, other_ad.lookup(*other_val));
                self.graph.borrow_mut().insert(&Term::Phi(
                    BlockId(unique_id as u32),
                    *self_val,
                    *other_val,
                    phi,
                ))
            };
            if *self_val == *other_val {
                var_to_val.insert(*var, *self_val);
            } else if let Some(entry) = static_phis.get_mut(var) {
                var_to_val.insert(*var, entry.0);
                let last_expr = make_phi();
                entry.1 = last_expr;
            } else {
                let static_phi = self.graph.borrow_mut().makeset();
                let last_expr = make_phi();
                static_phis.insert(*var, (static_phi, last_expr));
                var_to_val.insert(*var, static_phi);
                new_static_phi = true;
            }
        }

        let mut merged_ad = self_ad.widen(&other_ad, unique_id);
        if !new_static_phi {
            for (_, (static_phi, last_expr)) in static_phis {
                merged_ad.assign(*static_phi, merged_ad.lookup(*last_expr));
                self.graph.borrow_mut().merge(*static_phi, *last_expr);
            }
            static_phis_borrow.remove(&unique_id);
        }

        Self {
            var_to_val,
            num_params: self.num_params,
            graph: self.graph,
            static_phis: self.static_phis,
            ad: merged_ad,
        }
    }
}

impl AbstractDomain for () {
    type Variable = ClassId;
    type Value = ();
    type Expression = Term;

    fn bottom(&self) -> Self::Value {
        ()
    }

    fn forward_transfer(&mut self, _expr: &Self::Expression) -> Self::Value {
        ()
    }

    fn lookup(&self, _var: Self::Variable) -> Self::Value {
        ()
    }

    fn assign(&mut self, _var: Self::Variable, _val: Self::Value) {}

    fn branch(self, _cond: Self::Value) -> (Option<Self>, Option<Self>) {
        (Some(()), Some(()))
    }

    fn finish(self, _returned: Self::Value, _unique_id: usize) {
        ()
    }

    fn join(&self, _other: &Self, _unique_id: usize) -> Self {
        ()
    }

    fn widen(&self, _other: &Self, _unique_id: usize) -> Self {
        ()
    }
}
