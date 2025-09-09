pub trait AbstractDomain: Clone + PartialEq {
    type Variable;
    type Value;
    type Expression;

    fn bottom(&self) -> Self::Value;
    fn forward_transfer(&self, expr: &Self::Expression) -> Self::Value;
    fn lookup(&self, var: Self::Variable) -> Self::Value;
    fn assign(&mut self, var: Self::Variable, val: Self::Value);
    fn branch(self) -> (Self, Self);
    fn finish(self, returned: Self::Value);
    fn join(&self, other: &Self) -> Self;
    fn widen(&self, other: &Self, unique_id: usize) -> Self;
}
