use core::cmp::{max, min};
use std::collections::BTreeMap;

use imp::ast::ExpressionAST;
use imp::ast::Symbol;

use crate::domain::AbstractDomain;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    low: i32,
    high: i32,
}

impl Interval {
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

pub type IntervalDomain<Variable> = BTreeMap<Variable, Interval>;

impl AbstractDomain for IntervalDomain<Symbol> {
    type Variable = Symbol;
    type Value = Interval;
    type Expression = ExpressionAST;

    fn bottom(&self) -> Interval {
        Interval::bottom()
    }

    fn forward_transfer(&self, expr: &ExpressionAST) -> Interval {
        use ExpressionAST::*;
        match expr {
            NumberLiteral(lit) => Interval {
                low: *lit,
                high: *lit,
            },
            Variable(symbol) => self.lookup(*symbol),
            Call(_symbol, _expr) => todo!(),
            Add(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_add(rhs.low), lhs.high.checked_add(rhs.high))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }
            Subtract(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_sub(rhs.high), lhs.high.checked_sub(rhs.low))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }
            Multiply(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
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
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
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
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if lhs == rhs {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
            NotEquals(lhs, rhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if lhs == rhs {
                    Interval { low: 0, high: 0 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }
            Less(lhs, rhs) | Greater(rhs, lhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else if rhs.high <= lhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            },
            LessEquals(lhs, rhs) | GreaterEquals(rhs, lhs) => {
                let lhs = self.forward_transfer(lhs);
                let rhs = self.forward_transfer(rhs);
                if lhs.high <= rhs.low {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            },
        }
    }

    fn lookup(&self, var: Symbol) -> Interval {
        *self.get(&var).unwrap_or(&Interval::top())
    }

    fn assign(&mut self, var: Symbol, val: Interval) {
        self.insert(var, val);
    }

    fn branch(self) -> (Self, Self) {
        (self.clone(), self)
    }

    fn finish(self, _returned: Interval) {}

    fn join(&self, other: &Self) -> Self {
        let mut intervals = Self::new();
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        let mut m_self_pair = self_iter.next();
        let mut m_other_pair = other_iter.next();
        while let (Some(self_pair), Some(other_pair)) = (m_self_pair, m_other_pair) {
            if self_pair.0 < other_pair.0 {
                m_self_pair = self_iter.next();
            } else if self_pair.0 > other_pair.0 {
                m_other_pair = other_iter.next();
            } else {
                intervals.insert(*self_pair.0, self_pair.1.join(&other_pair.1));
            }
        }
        intervals
    }

    fn widen(&self, other: &Self, _unique_id: usize) -> Self {
        let mut intervals = Self::new();
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        let mut m_self_pair = self_iter.next();
        let mut m_other_pair = other_iter.next();
        while let (Some(self_pair), Some(other_pair)) = (m_self_pair, m_other_pair) {
            if self_pair.0 < other_pair.0 {
                m_self_pair = self_iter.next();
            } else if self_pair.0 > other_pair.0 {
                m_other_pair = other_iter.next();
            } else {
                intervals.insert(*self_pair.0, self_pair.1.widen(&other_pair.1));
            }
        }
        intervals
    }
}
