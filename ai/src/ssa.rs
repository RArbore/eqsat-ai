use core::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use bitvec::bitarr;
use bitvec::prelude::*;

use ds::egraph::{EGraph, ENode, Signature};
use ds::uf::ClassId;
use imp::ast::{ExpressionAST, Symbol};

use crate::domain::AbstractDomain;

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
pub struct ESSADomain<'a> {
    var_to_val: BTreeMap<Symbol, ClassId>,
    num_params: &'a Cell<u32>,
    graph: &'a RefCell<EGraph<Term>>,
}

impl<'a> ESSADomain<'a> {
    pub fn new(num_params: &'a Cell<u32>, graph: &'a RefCell<EGraph<Term>>) -> Self {
        Self {
            var_to_val: BTreeMap::new(),
            num_params,
            graph,
        }
    }
}

impl<'a> PartialEq for ESSADomain<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.var_to_val == other.var_to_val
    }
}

impl<'a> AbstractDomain for ESSADomain<'a> {
    type Variable = Symbol;
    type Value = ClassId;
    type Expression = ExpressionAST;

    fn bottom(&self) -> ClassId {
        let idx = self.num_params.get() as u32;
        self.num_params.set(idx + 1);
        let root = self.graph.borrow_mut().makeset();
        self.graph.borrow_mut().insert(&Term::Parameter(idx, root))
    }

    fn forward_transfer(&self, expr: &ExpressionAST) -> ClassId {
        let root = self.graph.borrow_mut().makeset();
        use ExpressionAST::*;
        match expr {
            NumberLiteral(lit) => self
                .graph
                .borrow_mut()
                .insert(&Term::Constant(*lit, root)),
            Variable(var) => self.lookup(*var),
            Add(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Add(lhs, rhs, root))
            }
            Subtract(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Subtract(lhs, rhs, root))
            }
            Multiply(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Multiply(lhs, rhs, root))
            }
            Divide(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Divide(lhs, rhs, root))
            }
            Modulo(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Modulo(lhs, rhs, root))
            }
            EqualsEquals(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::EqualsEquals(lhs, rhs, root))
            }
            NotEquals(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::NotEquals(lhs, rhs, root))
            }
            Less(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Less(lhs, rhs, root))
            }
            LessEquals(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::LessEquals(lhs, rhs, root))
            }
            Greater(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::Greater(lhs, rhs, root))
            }
            GreaterEquals(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                self.graph.borrow_mut().insert(&Term::GreaterEquals(lhs, rhs, root))
            }
            _ => todo!(),
        }
    }

    fn lookup(&self, var: Symbol) -> ClassId {
        self.var_to_val[&var]
    }

    fn assign(&mut self, var: Symbol, val: ClassId) {
        self.var_to_val.insert(var, val);
    }

    fn branch(self) -> (Self, Self) {
        todo!()
    }

    fn finish(self, _returned: ClassId, _unique_id: usize) {

    }

    fn join(&self, other: &Self) -> Self {
        todo!()
    }

    fn widen(&self, other: &Self, unique_id: usize) -> Self {
        todo!()
    }
}
