use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;
use std::iter::FromIterator;

/// Enables the ability to merge two items
pub trait Merge: Ord + Copy {
    /// Returns the result of merging self with other, if possible, and None otherwise.
    fn merge(self, other: Self) -> Option<Self>;
}

/// Enables the ability to coalesce items in a collection
/// Coalescence occurs when two items in the collection are merged
/// in lieu of inserting a new item
pub trait Coalesce: Sized + IntoIterator where Self::Item: Ord + Copy + Merge {
    /// Inserts or merges, if possible, the item into the collection at the given index.
    fn coalesce(&mut self, index: usize, el: Self::Item);
    /// Searches for the index to insert the element in order, then coalesces at the found index;
    fn search_coalesce(&mut self, start: usize, el: Self::Item) -> usize;

    /// Inserts, in order, the elements of an ordered iterable into self.
    /// Duplicate elements are not inserted.
    fn merge_coalesce<Iter>(&mut self, other: Iter)
        where Iter: IntoIterator<Item = Self::Item>
    {
        let mut idx = 0;
        for element in other {
            idx = self.search_coalesce(idx, element);
        }
    }
}

impl<T: Ord + Copy + Merge> Coalesce for Vec<T> {
    fn coalesce(&mut self, index: usize, el: T) {
        let merge = T::merge;
        if self.is_empty() {
            self.insert(index, el);
        } else if index == 0 {
            if let Some(coalesced) = merge(el, self[0]) {
                self[index] = coalesced;
            } else {
                self.insert(index, el);
            }
        } else if index == self.len() {
            if let Some(coalesced) = merge(self[index - 1], el) {
                self[index - 1] = coalesced;
            } else {
                self.insert(index, el);
            }
        } else {
            if let Some(coalesced) = merge(self[index - 1], el) {
                self[index - 1] = coalesced;
                if let Some(coalesced) = merge(self[index - 1], self[index]) {
                    self[index - 1] = coalesced;
                    self.remove(index);
                }
            } else if let Some(coalesced) = merge(el, self[index]) {
                self[index] = coalesced;
            } else {
                self.insert(index, el);
            }
        }
    }

    fn search_coalesce(&mut self, start: usize, el: T) -> usize {
        match self[start..].binary_search(&el) {
            Ok(idx) => start + idx,
            Err(idx) => {
                let idx = start + idx;
                self.coalesce(idx, el);
                idx
            }
        }
    }
}

/// A wrapper type that implements FromIterator in such a way that duplicate documents are
/// `merge_coalesce`d.
pub struct MergeCoalesceMap<K, V>(pub BTreeMap<K, V>);

impl<'a, 'b, K, V> FromIterator<(&'a K, &'b V)> for MergeCoalesceMap<K, V>
    where K: 'a + Ord + Clone,
          V: 'b + Coalesce + Clone,
          V::Item: Ord + Copy + Merge
{
    fn from_iter<It>(iterator: It) -> Self
        where It: IntoIterator<Item = (&'a K, &'b V)>
    {
        let mut map = BTreeMap::new();
        for (k, v) in iterator {
            match map.entry(k.clone()) {
                Vacant(entry) => {
                    entry.insert(v.clone());
                }
                Occupied(mut entry) => entry.get_mut().merge_coalesce(v.clone()),
            }
        }
        MergeCoalesceMap(map)
    }
}

impl<K, V> FromIterator<(K, V)> for MergeCoalesceMap<K, V>
    where K: Ord + Clone,
          V: Coalesce + Clone,
          V::Item: Ord + Copy + Merge
{
    fn from_iter<It>(iterator: It) -> Self
        where It: IntoIterator<Item = (K, V)>
    {
        let mut map = BTreeMap::new();
        for (k, v) in iterator {
            match map.entry(k) {
                Vacant(entry) => {
                    entry.insert(v);
                }
                Occupied(mut entry) => entry.get_mut().merge_coalesce(v.into_iter()),
            }
        }
        MergeCoalesceMap(map)
    }
}

macro_rules! impl_merge_tuples {
    ($tp:ident) => (
        impl Merge for ($tp, $tp) {
            fn merge(self, (begin2, end2): ($tp, $tp))  -> Option<($tp, $tp)> {
                let (begin1, end1) = self;
                assert!(begin2 >= begin1, "Input's begin must be >= self's begin");
                if end1 >= begin2 {
                    Some(if end1 < end2 { (begin1, end2) } else { (begin1, end1) })
                } else {
                    None
                }
            }
        }
    )
}

impl_merge_tuples!(isize);
impl_merge_tuples!(usize);
impl_merge_tuples!(u32);
impl_merge_tuples!(u16);
impl_merge_tuples!(u8);
impl_merge_tuples!(i32);
impl_merge_tuples!(i16);
impl_merge_tuples!(i8);

#[test]
fn test_coalesce_empty() {
    let mut v = vec![];
    v.coalesce(0, (0, 1));
    assert_eq!(v, [(0, 1)]);
}

#[test]
fn test_coalesce_first() {
    let mut v = vec![(1, 1)];
    v.coalesce(0, (0, 1));
    assert_eq!(v, [(0, 1)])
}

#[test]
fn test_coalesce_last() {
    let mut v = vec![(1, 1)];
    v.coalesce(1, (1, 2));
    assert_eq!(v, [(1, 2)])
}

#[test]
fn test_coalesce_both() {
    let mut v = vec![(1, 1), (2, 2)];
    v.coalesce(1, (1, 2));
    assert_eq!(v, [(1, 2)])
}

#[test]
fn test_coalesce_none() {
    let mut v = vec![(1, 1), (3, 3)];
    v.coalesce(1, (2, 2));
    assert_eq!(v, [(1, 1), (2, 2), (3, 3)])
}

#[test]
fn test_coalesce_twice() {
    let mut v = vec![];
    v.coalesce(0, (0, 1));
    v.coalesce(0, (-2, -1));
    v.coalesce(1, (-1, 0));
    assert_eq!(v, [(-2, 1)]);
}

#[test]
fn test_search_and_coalesce() {
    let mut v = vec![];
    for el in vec![(0, 1), (-2, -1), (-1, 0)] {
        let index = v.binary_search(&el).err().unwrap();
        v.coalesce(index, el);
    }
    assert_eq!(v, [(-2, 1)]);
}

#[test]
fn test_coalesce_subrange() {
    let mut v = vec![(0, 3)];
    v.coalesce(1, (1, 2));
    assert_eq!(v, [(0, 3)]);
}

#[test]
fn test_search_coalesce() {
    let mut v = vec![(0, 1), (2, 3), (4, 5), (6, 7)];
    assert_eq!(2, v.search_coalesce(1, (4, 5)));
}

#[test]
fn test_search_coalesce_2() {
    let mut v = vec![(0, 1), (2, 3), (4, 5), (6, 7)];
    assert_eq!(3, v.search_coalesce(1, (5, 6)));
    assert_eq!(v, [(0, 1), (2, 3), (4, 7)]);
}
