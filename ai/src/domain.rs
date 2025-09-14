use core::cell::RefCell;
use core::marker::PhantomData;
use std::collections::BTreeMap;

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

pub trait Lattice {
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
    var_to_val: BTreeMap<Variable, Value>,
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
        self.var_to_val.get(&var).unwrap_or(&Value::top()).clone()
    }

    fn assign(&mut self, var: Variable, val: Value) {
        self.var_to_val.insert(var, val);
    }

    fn branch(self, cond: Value) -> (Option<Self>, Option<Self>) {
        if cond.is_known_true(&self) {
            (Some(self), None)
        } else if cond.is_known_false(&self) {
            (None, Some(self))
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
