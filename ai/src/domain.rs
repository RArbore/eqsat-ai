use core::cell::RefCell;
use std::collections::BTreeMap;

pub trait AbstractDomain: Clone + PartialEq {
    type Variable;
    type Value;
    type Expression;

    fn bottom(&self) -> Self::Value;
    fn forward_transfer(&self, expr: &Self::Expression) -> Self::Value;
    fn lookup(&self, var: Self::Variable) -> Self::Value;
    fn assign(&mut self, var: Self::Variable, val: Self::Value);
    fn branch(self) -> (Self, Self);
    fn finish(self, returned: Self::Value, unique_id: usize);
    fn join(&self, other: &Self) -> Self;
    fn widen(&self, other: &Self, unique_id: usize) -> Self;
}

pub trait Lattice {
    fn top() -> Self;
    fn bottom() -> Self;
    fn join(&self, other: &Self) -> Self;
    fn meet(&self, other: &Self) -> Self;
    fn widen(&self, other: &Self) -> Self;
}

pub trait ForwardTransfer {
    type Variable;
    type Expression;

    fn forward_transfer<AD>(expr: &Self::Expression, ad: &AD) -> Self
    where
        AD: AbstractDomain<Variable = Self::Variable, Value = Self, Expression = Self::Expression>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatticeDomain<'a, Variable, Value> {
    var_to_val: BTreeMap<Variable, Value>,
    finished: &'a RefCell<BTreeMap<usize, Value>>,
}

impl<'a, Variable, Value> LatticeDomain<'a, Variable, Value> {
    pub fn new(finished: &'a RefCell<BTreeMap<usize, Value>>) -> Self {
        Self {
            var_to_val: BTreeMap::new(),
            finished,
        }
    }
}

impl<
    Variable: Clone + PartialEq + Ord,
    Value: Clone + PartialEq + Lattice + ForwardTransfer<Variable = Variable, Expression = Expression>,
    Expression,
> AbstractDomain for LatticeDomain<'_, Variable, Value>
{
    type Variable = Variable;
    type Value = Value;
    type Expression = Expression;

    fn bottom(&self) -> Value {
        Value::bottom()
    }

    fn forward_transfer(&self, expr: &Expression) -> Value {
        Value::forward_transfer(expr, self)
    }

    fn lookup(&self, var: Variable) -> Value {
        self.var_to_val.get(&var).unwrap_or(&Value::top()).clone()
    }

    fn assign(&mut self, var: Variable, val: Value) {
        self.var_to_val.insert(var, val);
    }

    fn branch(self) -> (Self, Self) {
        (self.clone(), self)
    }

    fn finish(self, returned: Value, unique_id: usize) {
        self.finished.borrow_mut().insert(unique_id, returned);
    }

    fn join(&self, other: &Self) -> Self {
        let mut intervals = BTreeMap::new();
        let mut self_iter = self.var_to_val.iter();
        let mut other_iter = other.var_to_val.iter();
        let mut m_self_pair = self_iter.next();
        let mut m_other_pair = other_iter.next();
        while let (Some(self_pair), Some(other_pair)) = (m_self_pair, m_other_pair) {
            if self_pair.0 < other_pair.0 {
                m_self_pair = self_iter.next();
            } else if self_pair.0 > other_pair.0 {
                m_other_pair = other_iter.next();
            } else {
                intervals.insert(self_pair.0.clone(), self_pair.1.join(&other_pair.1));
                m_self_pair = self_iter.next();
                m_other_pair = other_iter.next();
            }
        }
        Self {
            var_to_val: intervals,
            finished: self.finished,
        }
    }

    fn widen(&self, other: &Self, _unique_id: usize) -> Self {
        let mut intervals = BTreeMap::new();
        let mut self_iter = self.var_to_val.iter();
        let mut other_iter = other.var_to_val.iter();
        let mut m_self_pair = self_iter.next();
        let mut m_other_pair = other_iter.next();
        while let (Some(self_pair), Some(other_pair)) = (m_self_pair, m_other_pair) {
            if self_pair.0 < other_pair.0 {
                m_self_pair = self_iter.next();
            } else if self_pair.0 > other_pair.0 {
                m_other_pair = other_iter.next();
            } else {
                intervals.insert(self_pair.0.clone(), self_pair.1.widen(&other_pair.1));
                m_self_pair = self_iter.next();
                m_other_pair = other_iter.next();
            }
        }
        Self {
            var_to_val: intervals,
            finished: self.finished,
        }
    }
}
