use std::collections::btree_map::{BTreeMap, Keys};

/// A lazy iterator producing elements in the set intersection (in-order).
#[derive(Clone)]
pub struct Intersection<K, Iter: Iterator<Item=K>> {
    iters: Vec<Iter>
}

impl<K: Ord, V: Iterator<Item=K>> Iterator for Intersection<K, V> {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        let mut maximum = match self.iters.first_mut().map(Iterator::next) {
            Some(Some(k)) => k,
            _ => return None
        };

        // Where the maximum came from
        let mut skip_nth = 0;

        // Keep trying to...
        loop {
            let mut retry_with = None;

            // ...match all iters front element
            // with the chosen maximum
            for (i, iter) in self.iters.iter_mut().enumerate() {
                if i == skip_nth { continue; }
            
                match iter.find(|x| x >= &maximum) {
                    Some(val) => if val > maximum {
                        retry_with = Some(val);
                        skip_nth = i;
                        break;
                    },

                    // Intersection is empty
                    None => return None,
                }
            }

            match retry_with {
                Some(new_maximum) => maximum = new_maximum,
                None => return Some(maximum)
            }
        }
    }
}

/// An extension trait for slices of BTreeMaps that enables
/// computing intersections
pub trait BTreeMapExt {
    /// The type of the map's keys.
    type Key;

    /// The type of the map's keys iterator.
    type Iter: Iterator<Item=Self::Key>;

    /// Visits the values representing the intersection, in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use inverted_index::util::BTreeMapExt;
    ///
    /// let mut a = BTreeMap::new();
    /// a.insert(1, ());
    /// a.insert(2, ());
    ///
    /// let mut b = BTreeMap::new();
    /// b.insert(2, ());
    /// b.insert(3, ());
    ///
    /// let maps = &[a, b];
    /// let intersection: Vec<_> = maps.intersection().cloned().collect();
    /// assert_eq!(intersection, [2]);
    /// ```
    fn intersection(self) -> Intersection<Self::Key, Self::Iter>;
}

impl<'a, K: Ord, V> BTreeMapExt for &'a [BTreeMap<K, V>] {
    type Key = &'a K;
    type Iter = Keys<'a, K, V>;
    fn intersection(self) -> Intersection<&'a K, Keys<'a, K, V>> {
        Intersection{iters: self.iter().map(|map| map.keys()).collect() }
    }
}

#[test]
fn test_intersection_first_min() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, ());
    map1.insert(2, ());
    map1.insert(3, ());
    map1.insert(4, ());
    let mut map2 = BTreeMap::new();
    map2.insert(2, ());
    map2.insert(3, ());
    map2.insert(4, ());
    let mut map3 = BTreeMap::new();
    map3.insert(1, ());
    map3.insert(2, ());
    map3.insert(3, ());
    let maps = vec![map1, map2, map3];
    let maps = &*maps;
    let intersection: Vec<_> = maps.intersection().collect();
    assert_eq!(intersection, vec![&2, &3]);
}

#[test]
fn test_intersection_last_min() {
    let mut map1 = BTreeMap::new();
    map1.insert(2, ());
    map1.insert(3, ());
    map1.insert(4, ());
    map1.insert(5, ());
    let mut map2 = BTreeMap::new();
    map2.insert(2, ());
    map2.insert(3, ());
    map2.insert(4, ());
    let mut map3 = BTreeMap::new();
    map3.insert(1, ());
    map3.insert(2, ());
    map3.insert(3, ());
    let maps = vec![map1, map2, map3];
    let maps = &*maps;
    let intersection: Vec<_> = maps.intersection().collect();
    assert_eq!(intersection, vec![&2, &3]);
}
