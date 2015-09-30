/// Enables the ability to merge two items
pub trait Merge: Ord + Copy {
    /// Returns the result of merging self with other, if possible, and None otherwise.
    fn merge(self, other: Self) -> Option<Self>;
}

/// Enables the ability to coalesce items in a collection
/// Coalescence occurs when two items in the collection are merged
/// in lieu of inserting a new item
pub trait Coalesce<T: Ord + Copy + Merge> {
    /// Inserts or merges, if possible, the item into the collection at the given index.
    fn coalesce(&mut self, index: usize, el: T); 
    /// Searches for the index to insert the element in order, then coalesces at the found index;
    fn search_coalesce(&mut self, start: usize, el: T) -> usize;
}

impl<T: Ord + Copy + Merge> Coalesce<T> for Vec<T> {
    fn coalesce(&mut self, index: usize, el: T)  {
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
                self[index ] = coalesced;
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

macro_rules! impl_merge_tuples {
    ($tp:ident) => (
        impl Merge for ($tp, $tp) {
            fn merge(self, (x2, y2): ($tp, $tp))  -> Option<($tp, $tp)> {
                let (x1, y1) = self;
                if y1 >= x2 {
                    if y1 < y2 { Some((x1, y2)) } else { Some((x1, y1)) }
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

/// An extension trait for ordered vectors for inserting in order
pub trait InsertSorted {
    /// The type of ordered element in the collection
    type Element: Ord;
    /// Searches from the given index for the location to insert element, then inserts it.
    fn insert_sorted(&mut self, start: usize, element: Self::Element) -> usize;
}

impl<T: Ord> InsertSorted for Vec<T> {
    type Element = T;
    fn insert_sorted(&mut self, start: usize, element: T) -> usize {
        let idx = match self[start..].binary_search(&element) {
            Ok(idx) => idx,
            Err(idx) => idx,
        } + start;
        self.insert(idx, element);
        idx
    }
}

#[test]
fn test_insert_sorted() {
    let mut v = vec![0, 1, 2, 3, 4];
    v.insert_sorted(3, 5);
    assert_eq!(v, [0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_insert_sorted_front() {
    let mut v = vec![1, 2, 3, 4];
    v.insert_sorted(0, 0);
    assert_eq!(v, [0, 1, 2, 3, 4]);
}

