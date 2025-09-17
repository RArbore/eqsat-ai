use core::cell::RefCell;
use core::marker::PhantomData;
use std::collections::BTreeMap;

use ds::uf::{ClassId, UnionFind};

use crate::intersect_btree_maps;

pub trait AbstractDomain: Clone + PartialEq {
    type Variable;
    type Value: Clone;
    type Expression;

    fn bottom(&self) -> Self::Value;
    fn forward_transfer(&mut self, expr: &Self::Expression) -> Self::Value;
    fn lookup(&self, var: Self::Variable) -> Self::Value;
    fn assign(&mut self, var: Self::Variable, val: Self::Value);
    fn branch(self, cond: Self::Value) -> (Option<Self>, Option<Self>);
    fn finish(self, returned: Self::Value, unique_id: usize);
    fn join(&self, other: &Self, unique_id: usize) -> Self;
    fn widen(&self, other: &Self, unique_id: usize) -> Self;
}

pub trait Lattice: PartialEq {
    fn top() -> Self;
    fn bottom() -> Self;
    fn join(&self, other: &Self) -> Self;
    fn meet(&self, other: &Self) -> Self;
    fn widen(&self, other: &Self) -> Self;
}

pub trait ForwardTransfer<Variable, Expression> {
    fn forward_transfer<AD>(expr: &Expression, ad: &mut AD) -> Self
    where
        AD: AbstractDomain<Variable = Variable, Value = Self, Expression = Expression>;
    fn is_known_true<AD>(&self, ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Variable, Value = Self, Expression = Expression>;
    fn is_known_false<AD>(&self, ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Variable, Value = Self, Expression = Expression>;
}

#[derive(Debug)]
pub struct LatticeDomain<'a, Variable, Value, Expression> {
    pub(crate) var_to_val: BTreeMap<Variable, Value>,
    finished: &'a RefCell<BTreeMap<usize, Value>>,
    _phantom: PhantomData<Expression>,
}

impl<'a, Variable, Value, Expression> LatticeDomain<'a, Variable, Value, Expression> {
    pub fn new(finished: &'a RefCell<BTreeMap<usize, Value>>) -> Self {
        Self {
            var_to_val: BTreeMap::new(),
            finished,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Variable, Value, Expression> Clone for LatticeDomain<'a, Variable, Value, Expression>
where
    Variable: Clone,
    Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            var_to_val: self.var_to_val.clone(),
            finished: self.finished,
            _phantom: self._phantom.clone(),
        }
    }
}

impl<'a, Variable, Value, Expression> PartialEq for LatticeDomain<'a, Variable, Value, Expression>
where
    Variable: PartialEq,
    Value: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.var_to_val == other.var_to_val
    }
}

impl<Variable, Expression, Value> AbstractDomain for LatticeDomain<'_, Variable, Value, Expression>
where
    Variable: Clone + PartialEq + Ord,
    Value: Clone + PartialEq + Lattice + ForwardTransfer<Variable, Expression>,
{
    type Variable = Variable;
    type Value = Value;
    type Expression = Expression;

    fn bottom(&self) -> Value {
        Value::bottom()
    }

    fn forward_transfer(&mut self, expr: &Expression) -> Value {
        Value::forward_transfer(expr, self)
    }

    fn lookup(&self, var: Variable) -> Value {
        self.var_to_val
            .get(&var)
            .map(|val| val.clone())
            .unwrap_or_else(|| Value::top())
    }

    fn assign(&mut self, var: Variable, val: Value) {
        self.var_to_val.insert(var, val);
    }

    fn branch(self, cond: Value) -> (Option<Self>, Option<Self>) {
        if cond.is_known_false(&self) {
            (None, Some(self))
        } else if cond.is_known_true(&self) {
            (Some(self), None)
        } else {
            (Some(self.clone()), Some(self))
        }
    }

    fn finish(self, returned: Value, unique_id: usize) {
        self.finished.borrow_mut().insert(unique_id, returned);
    }

    fn join(&self, other: &Self, _unique_id: usize) -> Self {
        let mut intervals = BTreeMap::new();
        for (var, self_val, other_val) in intersect_btree_maps(&self.var_to_val, &other.var_to_val)
        {
            intervals.insert(var.clone(), self_val.join(other_val));
        }
        Self {
            var_to_val: intervals,
            finished: self.finished,
            _phantom: PhantomData,
        }
    }

    fn widen(&self, other: &Self, _unique_id: usize) -> Self {
        let mut intervals = BTreeMap::new();
        for (var, self_val, other_val) in intersect_btree_maps(&self.var_to_val, &other.var_to_val)
        {
            intervals.insert(var.clone(), self_val.widen(other_val));
        }
        Self {
            var_to_val: intervals,
            finished: self.finished,
            _phantom: PhantomData,
        }
    }
}

pub trait UnderstandsEquality: AbstractDomain<Variable = ClassId> {
    fn merge(&mut self, a: ClassId, b: ClassId) -> (Self::Value, bool);
    fn dom(&self) -> impl Iterator<Item = ClassId> + '_;

    fn canonicalize(&mut self, uf: &mut UnionFind) {
        loop {
            let mut changed = false;
            let dom: Vec<_> = self.dom().collect();
            for id in &dom {
                let canon = uf.find(*id);
                if *id != canon {
                    changed = self.merge(*id, canon).1 || changed;
                }
            }
            if !changed {
                break;
            }
        }
    }
}

impl<Expression, Value> UnderstandsEquality for LatticeDomain<'_, ClassId, Value, Expression>
where
    Value: Clone + PartialEq + Lattice + ForwardTransfer<ClassId, Expression>,
{
    fn merge(&mut self, a: ClassId, b: ClassId) -> (Self::Value, bool) {
        match (self.var_to_val.get(&a), self.var_to_val.get(&b)) {
            (None, None) => (Value::bottom(), false),
            (None, Some(val)) => {
                let val = val.clone();
                self.assign(a, val.clone());
                (val, true)
            }
            (Some(val), None) => {
                let val = val.clone();
                self.assign(b, val.clone());
                (val.clone(), true)
            }
            (Some(a_val), Some(b_val)) => {
                let new_val = a_val.meet(b_val);
                let old_a = self.var_to_val.insert(a, new_val.clone()).unwrap();
                let old_b = self.var_to_val.insert(b, new_val.clone()).unwrap();
                let changed = new_val != old_a || new_val != old_b;
                (new_val, changed)
            }
        }
    }

    fn dom(&self) -> impl Iterator<Item = ClassId> + '_ {
        self.var_to_val.keys().map(|id| *id)
    }
}
