use imp::ast::ExpressionAST;
use imp::ast::Symbol;

use crate::domain::{AbstractDomain, ForwardTransfer, Lattice};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Concrete {
    Bottom,
    Value(i32),
    Top,
}

impl Lattice for Concrete {
    fn top() -> Self {
        Self::Top
    }

    fn bottom() -> Self {
        Self::Bottom
    }

    fn join(&self, other: &Self) -> Self {
        match (*self, *other) {
            (Concrete::Top, other) | (other, Concrete::Top) => other,
            _ if *self == *other => *self,
            _ => Concrete::Bottom,
        }
    }

    fn meet(&self, other: &Self) -> Self {
        match (*self, *other) {
            (Concrete::Bottom, other) | (other, Concrete::Bottom) => other,
            _ if *self == *other => *self,
            _ => Concrete::Top,
        }
    }

    fn widen(&self, other: &Self) -> Self {
        *other
    }
}

impl ForwardTransfer<Symbol, ExpressionAST> for Concrete {
    fn forward_transfer<AD>(expr: &ExpressionAST, ad: &mut AD) -> Self
    where
        AD: AbstractDomain<Variable = Symbol, Value = Self, Expression = ExpressionAST>,
    {
        let mut eval = |lhs, rhs, func: &dyn Fn(i32, i32) -> Option<i32>| {
            let lhs = Self::forward_transfer(lhs, ad);
            let rhs = Self::forward_transfer(rhs, ad);
            match (lhs, rhs) {
                (Concrete::Top, _) | (_, Concrete::Top) => Concrete::Top,
                (Concrete::Bottom, _) | (_, Concrete::Bottom) => Concrete::Top,
                (Concrete::Value(lhs), Concrete::Value(rhs)) => {
                    if let Some(value) = func(lhs, rhs) {
                        Concrete::Value(value)
                    } else {
                        Concrete::Top
                    }
                },
            }
        };
        match expr {
            ExpressionAST::NumberLiteral(lit) => Concrete::Value(*lit),
            ExpressionAST::Variable(symbol) => ad.lookup(*symbol),
            ExpressionAST::Call(..) => todo!(),
            ExpressionAST::Add(lhs, rhs) => eval(lhs, rhs, &|a, b| Some(a.wrapping_add(b))),
            ExpressionAST::Subtract(lhs, rhs) => eval(lhs, rhs, &|a, b| Some(a.wrapping_sub(b))),
            ExpressionAST::Multiply(lhs, rhs) => eval(lhs, rhs, &|a, b| Some(a.wrapping_mul(b))),
            ExpressionAST::Divide(lhs, rhs) => eval(lhs, rhs, &|a, b| a.checked_div(b)),
            ExpressionAST::Modulo(lhs, rhs) => eval(lhs, rhs, &|a, b| a.checked_rem(b)),
            ExpressionAST::EqualsEquals(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a == b) as i32)),
            ExpressionAST::NotEquals(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a != b) as i32)),
            ExpressionAST::Less(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a < b) as i32)),
            ExpressionAST::LessEquals(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a <= b) as i32)),
            ExpressionAST::Greater(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a > b) as i32)),
            ExpressionAST::GreaterEquals(lhs, rhs) => eval(lhs, rhs, &|a, b| Some((a >= b) as i32)),
        }
    }

    fn is_known_true<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Symbol, Value = Self, Expression = ExpressionAST>,
    {
        match *self {
            Concrete::Top => true,
            Concrete::Value(val) => val != 0,
            Concrete::Bottom => false,
        }
    }

    fn is_known_false<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Symbol, Value = Self, Expression = ExpressionAST>,
    {
        match *self {
            Concrete::Top => true,
            Concrete::Value(val) => val == 0,
            Concrete::Bottom => false,
        }
    }
}
