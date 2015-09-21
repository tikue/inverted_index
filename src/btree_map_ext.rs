use core::iter::Peekable;
use std::cmp::Ordering::*;
use std::collections::btree_map::{BTreeMap, Keys};

/// A lazy iterator producing elements in the set intersection (in-order).
pub struct Intersection<'a, K:'a, V:'a> {
    iters: Vec<Keys<'a, K, V>>,
}

impl<'a, K, V> Clone for Intersection<'a, K, V> {
    fn clone(&self) -> Intersection<'a, K, V> {
        Intersection { iters: self.iters.clone() }
    }
}

impl<'a, K: Ord, V> Iterator for Intersection<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        match &mut *self.iters {
            [] => None,
            [ref mut iter] => iter.next(),
            [ref mut first, ref mut rest..] => {
                /*
                loop {
                    let all_equal = true;
                    for iter in &mut rest {
                        match  {
                            None          => return None,
                            Some(Less)    => { self.a.next(); }
                            Some(Equal)   => { self.b.next(); return self.a.next() }
                            Some(Greater) => { self.b.next(); }
                        }
                    }
                    let o_cmp = match (self.a.peek(), self.b.peek()) {
                        (None    , _       ) => None,
                        (_       , None    ) => None,
                        (Some(a1), Some(b1)) => Some(a1.cmp(b1)),
                    };
                }
                */
                None
            }
        }
    }
}

pub trait BTreeMapExt<'a> {
    /// Visits the values representing the intersection, in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeSet;
    ///
    /// let mut a = BTreeSet::new();
    /// a.insert(1);
    /// a.insert(2);
    ///
    /// let mut b = BTreeSet::new();
    /// b.insert(2);
    /// b.insert(3);
    ///
    /// let intersection: Vec<_> = a.intersection(&b).cloned().collect();
    /// assert_eq!(intersection, [2]);
    /// ```
    type Key;
    type Value;
    fn intersection(&'a self) -> Intersection<'a, Self::Key, Self::Value>;
}

impl<'a, K: Ord, V> BTreeMapExt<'a> for &'a [&'a BTreeMap<K, V>] {
    type Key = K;
    type Value = V;
    fn intersection(&'a self) -> Intersection<'a, K, V> {
        Intersection{iters: self.iter().map(|map| map.keys()).collect() }
    }
}
