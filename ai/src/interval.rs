use core::cmp::{max, min};

use ds::uf::ClassId;
use imp::ast::ExpressionAST;
use imp::ast::Symbol;

use crate::domain::{AbstractDomain, ForwardTransfer, Lattice};
use crate::essa::Term;

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

impl ForwardTransfer<Symbol, ExpressionAST> for Interval {
    fn forward_transfer<AD>(expr: &ExpressionAST, ad: &mut AD) -> Self
    where
        AD: AbstractDomain<Value = Self, Variable = Symbol, Expression = ExpressionAST>,
    {
        let mut eval = |lhs, rhs, func: &dyn Fn(Interval, Interval) -> Interval| {
            let lhs = ad.forward_transfer(lhs);
            let rhs = ad.forward_transfer(rhs);
            func(lhs, rhs)
        };
        use ExpressionAST::*;
        match expr {
            NumberLiteral(lit) => Interval {
                low: *lit,
                high: *lit,
            },
            Variable(symbol) => ad.lookup(*symbol),
            Call(..) => todo!(),
            Add(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_add(rhs.low), lhs.high.checked_add(rhs.high))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }),
            Subtract(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_sub(rhs.high), lhs.high.checked_sub(rhs.low))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }),
            Multiply(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
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
            }),
            Divide(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                let low_low = lhs.low / if rhs.low != 0 { rhs.low } else { 1 };
                let low_high = lhs.low / if rhs.high != 0 { rhs.high } else { -1 };
                let high_low = lhs.high / if rhs.low != 0 { rhs.low } else { 1 };
                let high_high = lhs.high / if rhs.high != 0 { rhs.high } else { -1 };
                Interval {
                    low: low_low.min(low_high).min(high_low).min(high_high),
                    high: low_low.max(low_high).max(high_low).max(high_high),
                }
            }),
            Modulo(_lhs, _rhs) => todo!(),
            EqualsEquals(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                if lhs == rhs {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }),
            NotEquals(lhs, rhs) => eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                if lhs == rhs {
                    Interval { low: 0, high: 0 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }),
            Less(lhs, rhs) | Greater(rhs, lhs) => {
                eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                    if lhs.high < rhs.low {
                        Interval { low: 1, high: 1 }
                    } else if rhs.high <= lhs.low {
                        Interval { low: 0, high: 0 }
                    } else {
                        Interval { low: 0, high: 1 }
                    }
                })
            }
            LessEquals(lhs, rhs) | GreaterEquals(rhs, lhs) => {
                eval(lhs, rhs, &|lhs: Interval, rhs: Interval| {
                    if lhs.high <= rhs.low {
                        Interval { low: 1, high: 1 }
                    } else if rhs.high < lhs.low {
                        Interval { low: 0, high: 0 }
                    } else {
                        Interval { low: 0, high: 1 }
                    }
                })
            }
        }
    }

    fn is_known_true<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Symbol, Value = Self, Expression = ExpressionAST>,
    {
        self.low >= 1 || self.high <= -1
    }

    fn is_known_false<AD>(&self, _ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = Symbol, Value = Self, Expression = ExpressionAST>,
    {
        *self == Interval { low: 0, high: 0 } || *self == Interval::top()
    }
}

impl ForwardTransfer<ClassId, Term> for Interval {
    fn forward_transfer<AD>(term: &Term, ad: &mut AD) -> Self
    where
        AD: AbstractDomain<Variable = ClassId, Value = Self, Expression = Term>,
    {
        let eval = |lhs, rhs, func: &dyn Fn(Interval, Interval) -> Interval| {
            func(ad.lookup(lhs), ad.lookup(rhs))
        };
        use Term::*;
        match term {
            Constant(cons, _) => Interval {
                low: *cons,
                high: *cons,
            },
            Term::Parameter(_, root) | Phi(_, _, _, root) => ad.lookup(*root),
            Add(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_add(rhs.low), lhs.high.checked_add(rhs.high))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }),
            Subtract(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                if let (Some(low), Some(high)) =
                    (lhs.low.checked_sub(rhs.high), lhs.high.checked_sub(rhs.low))
                {
                    Interval { low, high }
                } else {
                    Interval::bottom()
                }
            }),
            Multiply(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
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
            }),
            Divide(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                let low_low = lhs.low / if rhs.low != 0 { rhs.low } else { 1 };
                let low_high = lhs.low / if rhs.high != 0 { rhs.high } else { -1 };
                let high_low = lhs.high / if rhs.low != 0 { rhs.low } else { 1 };
                let high_high = lhs.high / if rhs.high != 0 { rhs.high } else { -1 };
                Interval {
                    low: low_low.min(low_high).min(high_low).min(high_high),
                    high: low_low.max(low_high).max(high_low).max(high_high),
                }
            }),
            Modulo(_lhs, _rhs, _) => todo!(),
            EqualsEquals(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                if lhs == rhs {
                    Interval { low: 1, high: 1 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 0, high: 0 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }),
            NotEquals(lhs, rhs, _) => eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                if lhs == rhs {
                    Interval { low: 0, high: 0 }
                } else if rhs.high < lhs.low || lhs.high < rhs.low {
                    Interval { low: 1, high: 1 }
                } else {
                    Interval { low: 0, high: 1 }
                }
            }),
            Less(lhs, rhs, _) | Greater(rhs, lhs, _) => {
                eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                    if lhs.high < rhs.low {
                        Interval { low: 1, high: 1 }
                    } else if rhs.high <= lhs.low {
                        Interval { low: 0, high: 0 }
                    } else {
                        Interval { low: 0, high: 1 }
                    }
                })
            }
            LessEquals(lhs, rhs, _) | GreaterEquals(rhs, lhs, _) => {
                eval(*lhs, *rhs, &|lhs: Interval, rhs: Interval| {
                    if lhs.high <= rhs.low {
                        Interval { low: 1, high: 1 }
                    } else if rhs.high < lhs.low {
                        Interval { low: 0, high: 0 }
                    } else {
                        Interval { low: 0, high: 1 }
                    }
                })
            }
        }
    }

    fn is_known_true<AD>(&self, ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = ClassId, Value = Self, Expression = Term>,
    {
        self.low >= 1 || self.high <= -1
    }

    fn is_known_false<AD>(&self, ad: &AD) -> bool
    where
        AD: AbstractDomain<Variable = ClassId, Value = Self, Expression = Term>,
    {
        *self == Interval { low: 0, high: 0 } || *self == Interval::top()
    }
}
