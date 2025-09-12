pub mod concrete;
pub mod domain;
pub mod imp;
pub mod interval;
pub mod ssa;

use std::collections::BTreeMap;
use std::collections::btree_map::Iter;

pub(crate) fn intersect_btree_maps<'a, K: Ord, V1, V2>(
    a: &'a BTreeMap<K, V1>,
    b: &'a BTreeMap<K, V2>,
) -> impl Iterator<Item = (&'a K, &'a V1, &'a V2)> + 'a {
    IntersectBTreeMapIterator {
        a_iter: a.iter(),
        b_iter: b.iter(),
    }
}

struct IntersectBTreeMapIterator<'a, K, V1, V2> {
    a_iter: Iter<'a, K, V1>,
    b_iter: Iter<'a, K, V2>,
}

impl<'a, K: Ord, V1, V2> Iterator for IntersectBTreeMapIterator<'a, K, V1, V2> {
    type Item = (&'a K, &'a V1, &'a V2);

    fn next(&mut self) -> Option<Self::Item> {
        let (Some(mut a_pair), Some(mut b_pair)) = (self.a_iter.next(), self.b_iter.next()) else {
            return None;
        };
        loop {
            if a_pair.0 < b_pair.0 {
                a_pair = self.a_iter.next()?;
            } else if b_pair.0 < a_pair.0 {
                b_pair = self.b_iter.next()?;
            } else {
                return Some((a_pair.0, a_pair.1, b_pair.1));
            }
        }
    }
}
