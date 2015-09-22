pub trait Merge: Ord + Copy {
    fn merge(self, other: Self) -> Option<Self>;
}

pub trait Coalesce<T: Ord + Copy + Merge> {
    fn coalesce(&mut self, index: usize, el: T); 
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
