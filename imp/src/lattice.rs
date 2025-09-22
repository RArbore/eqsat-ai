use ds::table::Value;

pub trait JoinSemilattice {
    fn bottom() -> Self;
    fn join(&self, other: &Self) -> Self;
    fn leq(&self, other: &Self) -> bool;
}

pub trait MeetSemilattice {
    fn top() -> Self;
    fn meet(&self, other: &Self) -> Self;
    fn leq(&self, other: &Self) -> bool;
}

pub trait Widenable {
    fn widen(&self, other: &Self) -> Self;
}

pub trait Lattice {
    fn bottom() -> Self;
    fn top() -> Self;
    fn leq(&self, other: &Self) -> bool;
    fn join(&self, other: &Self) -> Self;
    fn meet(&self, other: &Self) -> Self;
}

impl<T> Lattice for T
where
    T: JoinSemilattice + MeetSemilattice,
{
    fn bottom() -> Self {
        T::bottom()
    }

    fn top() -> Self {
        T::top()
    }

    fn leq(&self, other: &Self) -> bool {
        let join = JoinSemilattice::leq(self, other);
        let meet = MeetSemilattice::leq(self, other);
        assert_eq!(join, meet);
        join
    }

    fn join(&self, other: &Self) -> Self {
        self.join(other)
    }

    fn meet(&self, other: &Self) -> Self {
        self.meet(other)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reachability {
    Unreachable,
    Reachable,
}

impl JoinSemilattice for Reachability {
    fn bottom() -> Self {
        Reachability::Reachable
    }

    fn join(&self, other: &Self) -> Self {
        use Reachability::*;
        match (self, other) {
            (Reachable, Reachable) => Reachable,
            _ => Unreachable,
        }
    }

    fn leq(&self, other: &Self) -> bool {
        use Reachability::*;
        *self == Reachable || *other == Unreachable
    }
}

impl MeetSemilattice for Reachability {
    fn top() -> Self {
        Reachability::Unreachable
    }

    fn meet(&self, other: &Self) -> Self {
        use Reachability::*;
        match (self, other) {
            (Unreachable, Unreachable) => Unreachable,
            _ => Reachable,
        }
    }

    fn leq(&self, other: &Self) -> bool {
        JoinSemilattice::leq(self, other)
    }
}

impl Widenable for Reachability {
    fn widen(&self, other: &Self) -> Self {
        MeetSemilattice::meet(self, other)
    }
}

impl From<Value> for Reachability {
    fn from(value: Value) -> Self {
        match value {
            0 => Reachability::Unreachable,
            1 => Reachability::Reachable,
            _ => panic!(),
        }
    }
}

impl From<Reachability> for Value {
    fn from(value: Reachability) -> Self {
        match value {
            Reachability::Unreachable => 0,
            Reachability::Reachable => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constant {
    Top,
    Value(i32),
    Bottom,
}

impl JoinSemilattice for Constant {
    fn bottom() -> Self {
        Constant::Bottom
    }

    fn join(&self, other: &Self) -> Self {
        match (self, other) {
            (Constant::Bottom, x) | (x, Constant::Bottom) => *x,
            (x, y) if x == y => *x,
            _ => Constant::Top,
        }
    }

    fn leq(&self, other: &Self) -> bool {
        match (self, other) {
            (Constant::Bottom, _) | (_, Constant::Top) => true,
            (x, y) if x == y => true,
            _ => false,
        }
    }
}

impl MeetSemilattice for Constant {
    fn top() -> Self {
        Constant::Top
    }

    fn meet(&self, other: &Self) -> Self {
        match (self, other) {
            (Constant::Top, x) | (x, Constant::Top) => *x,
            (x, y) if x == y => *x,
            _ => Constant::Bottom,
        }
    }

    fn leq(&self, other: &Self) -> bool {
        JoinSemilattice::leq(self, other)
    }
}

impl Widenable for Constant {
    fn widen(&self, other: &Self) -> Self {
        MeetSemilattice::meet(self, other)
    }
}

impl From<[Value; 2]> for Constant {
    fn from(value: [Value; 2]) -> Self {
        match value[0] {
            0 => Constant::Value(value[1].cast_signed()),
            1 => Constant::Top,
            2 => Constant::Bottom,
            _ => panic!(),
        }
    }
}

impl From<Constant> for [Value; 2] {
    fn from(value: Constant) -> Self {
        match value {
            Constant::Value(val) => [0, val.cast_unsigned()],
            Constant::Top => [1, 0],
            Constant::Bottom => [2, 0],
        }
    }
}
