use core::cmp::{max, min};

use imp::ast::ExpressionAST;
use imp::ast::Symbol;

use crate::domain::{AbstractDomain, ForwardTransfer, Lattice};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub low: i32,
    pub high: i32,
}

impl Lattice for Interval {
    fn top() -> Self {
        Self {
            low: i32::MAX,
            high: i32::MIN,
        }
    }

    fn bottom() -> Self {
        Self {
            low: i32::MIN,
            high: i32::MAX,
        }
    }

    fn join(&self, other: &Interval) -> Self {
        Self {
            low: min(self.low, other.low),
            high: max(self.high, other.high),
        }
    }

    fn meet(&self, other: &Interval) -> Self {
        let met = Self {
            low: max(self.low, other.low),
            high: min(self.high, other.high),
        };
        if met.low > met.high { Self::top() } else { met }
    }

    fn widen(&self, other: &Interval) -> Self {
        Self {
            low: if self.low <= other.low {
                self.low
            } else {
                i32::MIN
            },
            high: if self.high >= other.high {
                self.high
            } else {
                i32::MAX
            },
        }
    }
}

impl ForwardTransfer for Interval {
    type Variable = Symbol;
    type Expression = ExpressionAST;

    fn forward_transfer<AD>(expr: &ExpressionAST, ad: &AD) -> Self
    where
        AD: AbstractDomain<Value = Self, Variable = Symbol, Expression = ExpressionAST>,
    {
        use ExpressionAST::*;
        match expr {
            NumberLiteral(lit) => Interval {
                low: *lit,
                high: *lit,
            },
            Variable(symbol) => ad.lookup(*symbol),
            Call(..) => todo!(),
            Add(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_add(rhs.low), lhs.high.checked_add(rhs.high))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }
            Subtract(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_sub(rhs.high), lhs.high.checked_sub(rhs.low))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }
            Multiply(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if let (Some(low_low), Some(low_high), Some(high_low), Some(high_high)) = (
                    lhs.low.checked_mul(rhs.low),
                    lhs.low.checked_mul(rhs.high),
                    lhs.high.checked_mul(rhs.low),
                    lhs.high.checked_mul(rhs.high),
                ) {
                    Interval {
                        low: low_low.min(low_high).min(high_low).min(high_high),
                        high: low_low.max(low_high).max(high_low).max(high_high),
                    }
                } else {
                    Interval::bottom()
                }
            }
            Divide(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                let low_low = lhs.low / if rhs.low != 0 { rhs.low } else { 1 };
                let low_high = lhs.low / if rhs.high != 0 { rhs.high } else { -1 };
                let high_low = lhs.high / if rhs.low != 0 { rhs.low } else { 1 };
                let high_high = lhs.high / if rhs.high != 0 { rhs.high } else { -1 };
                Interval {
                    low: low_low.min(low_high).min(high_low).min(high_high),
                    high: low_low.max(low_high).max(high_low).max(high_high),
                }
            }
            Modulo(_lhs, _rhs) => todo!(),
            EqualsEquals(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if lhs == rhs {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
            NotEquals(lhs, rhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if lhs == rhs {
                    Interval { low: 0, high: 0 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
            Less(lhs, rhs) | Greater(rhs, lhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else if rhs.high <= lhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
            LessEquals(lhs, rhs) | GreaterEquals(rhs, lhs) => {
                let lhs = ad.forward_transfer(lhs);
                let rhs = ad.forward_transfer(rhs);
                if lhs.high <= rhs.low {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
        }
    }

    fn is_known_true<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Self::Variable, Value = Self, Expression = Self::Expression>,
    {
        *self == Interval { low: 1, high: 1 }
    }

    fn is_known_false<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Self::Variable, Value = Self, Expression = Self::Expression>,
    {
        *self == Interval { low: 0, high: 0 }
    }
}
