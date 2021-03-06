// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::thread;
use std::cell::RefCell;
use CHashMap;

#[test]
fn test_create_capacity_zero() {
    let mut m = CHashMap::with_capacity(0);

    assert!(m.insert(1, 1).is_none());

    assert!(m.contains_key(&1));
    assert!(!m.contains_key(&0));
}

#[test]
fn test_insert() {
    let mut m = CHashMap::new();
    assert_eq!(m.len(), 0);
    assert!(m.insert(1, 2).is_none());
    assert_eq!(m.len(), 1);
    assert!(m.insert(2, 4).is_none());
    assert_eq!(m.len(), 2);
    assert_eq!(*m.get(&1).unwrap(), 2);
    assert_eq!(*m.get(&2).unwrap(), 4);
}

thread_local! { static DROP_VECTOR: RefCell<Vec<isize>> = RefCell::new(Vec::new()) }

#[derive(Hash, PartialEq, Eq)]
struct Dropable {
    k: usize
}

impl Dropable {
    fn new(k: usize) -> Dropable {
        DROP_VECTOR.with(|slot| {
            slot.borrow_mut()[k] += 1;
        });

        Dropable { k: k }
    }
}

impl Drop for Dropable {
    fn drop(&mut self) {
        DROP_VECTOR.with(|slot| {
            slot.borrow_mut()[self.k] -= 1;
        });
    }
}

impl Clone for Dropable {
    fn clone(&self) -> Dropable {
        Dropable::new(self.k)
    }
}

#[test]
fn test_drops() {
    DROP_VECTOR.with(|slot| {
        *slot.borrow_mut() = vec![0; 200];
    });

    {
        let mut m = CHashMap::new();

        DROP_VECTOR.with(|v| {
            for i in 0..200 {
                assert_eq!(v.borrow()[i], 0);
            }
        });

        for i in 0..100 {
            let d1 = Dropable::new(i);
            let d2 = Dropable::new(i+100);
            m.insert(d1, d2);
        }

        DROP_VECTOR.with(|v| {
            for i in 0..200 {
                assert_eq!(v.borrow()[i], 1);
            }
        });

        for i in 0..50 {
            let k = Dropable::new(i);
            let v = m.remove(&k);

            assert!(v.is_some());

            DROP_VECTOR.with(|v| {
                assert_eq!(v.borrow()[i], 1);
                assert_eq!(v.borrow()[i+100], 1);
            });
        }

        DROP_VECTOR.with(|v| {
            for i in 0..50 {
                assert_eq!(v.borrow()[i], 0);
                assert_eq!(v.borrow()[i+100], 0);
            }

            for i in 50..100 {
                assert_eq!(v.borrow()[i], 1);
                assert_eq!(v.borrow()[i+100], 1);
            }
        });
    }

    DROP_VECTOR.with(|v| {
        for i in 0..200 {
            assert_eq!(v.borrow()[i], 0);
        }
    });
}

#[test]
fn test_move_iter_drops() {
    DROP_VECTOR.with(|v| {
        *v.borrow_mut() = vec![0; 200];
    });

    let hm = {
        let mut hm = CHashMap::new();

        DROP_VECTOR.with(|v| {
            for i in 0..200 {
                assert_eq!(v.borrow()[i], 0);
            }
        });

        for i in 0..100 {
            let d1 = Dropable::new(i);
            let d2 = Dropable::new(i+100);
            hm.insert(d1, d2);
        }

        DROP_VECTOR.with(|v| {
            for i in 0..200 {
                assert_eq!(v.borrow()[i], 1);
            }
        });

        hm
    };

    // By the way, ensure that cloning doesn't screw up the dropping.
    drop(hm.clone());

    {
        let mut half = hm.into_iter().take(50);

        DROP_VECTOR.with(|v| {
            for i in 0..200 {
                assert_eq!(v.borrow()[i], 1);
            }
        });

        for _ in half.by_ref() {}

        DROP_VECTOR.with(|v| {
            let nk = (0..100).filter(|&i| {
                v.borrow()[i] == 1
            }).count();

            let nv = (0..100).filter(|&i| {
                v.borrow()[i+100] == 1
            }).count();

            assert_eq!(nk, 50);
            assert_eq!(nv, 50);
        });
    };

    DROP_VECTOR.with(|v| {
        for i in 0..200 {
            assert_eq!(v.borrow()[i], 0);
        }
    });
}

#[test]
fn test_empty_pop() {
    let mut m: CHashMap<isize, bool> = CHashMap::new();
    assert_eq!(m.remove(&0), None);
}

#[test]
fn test_lots_of_insertions() {
    let mut m = CHashMap::new();

    // Try this a few times to make sure we never screw up the hashmap's
    // internal state.
    for _ in 0..10 {
        assert!(m.is_empty());

        for i in 1..1001 {
            assert!(m.insert(i, i).is_none());

            for j in 1..i+1 {
                let r = m.get(&j);
                assert_eq!(r, Some(&j));
            }

            for j in i+1..1001 {
                let r = m.get(&j);
                assert_eq!(r, None);
            }
        }

        for i in 1001..2001 {
            assert!(!m.contains_key(&i));
        }

        // remove forwards
        for i in 1..1001 {
            assert!(m.remove(&i).is_some());

            for j in 1..i+1 {
                assert!(!m.contains_key(&j));
            }

            for j in i+1..1001 {
                assert!(m.contains_key(&j));
            }
        }

        for i in 1..1001 {
            assert!(!m.contains_key(&i));
        }

        for i in 1..1001 {
            assert!(m.insert(i, i).is_none());
        }

        // remove backwards
        for i in (1..1001).rev() {
            assert!(m.remove(&i).is_some());

            for j in i..1001 {
                assert!(!m.contains_key(&j));
            }

            for j in 1..i {
                assert!(m.contains_key(&j));
            }
        }
    }
}

#[test]
fn test_find_mut() {
    let mut m = CHashMap::new();
    assert!(m.insert(1, 12).is_none());
    assert!(m.insert(2, 8).is_none());
    assert!(m.insert(5, 14).is_none());
    let new = 100;
    match m.get_mut(&5) {
        None => panic!(), Some(x) => *x = new
    }
    assert_eq!(m.get(&5), Some(&new));
}

#[test]
fn test_insert_overwrite() {
    let mut m = CHashMap::new();
    assert!(m.insert(1, 2).is_none());
    assert_eq!(*m.get(&1).unwrap(), 2);
    assert!(!m.insert(1, 3).is_none());
    assert_eq!(*m.get(&1).unwrap(), 3);
}

#[test]
fn test_insert_conflicts() {
    let mut m = CHashMap::with_capacity(4);
    assert!(m.insert(1, 2).is_none());
    assert!(m.insert(5, 3).is_none());
    assert!(m.insert(9, 4).is_none());
    assert_eq!(*m.get(&9).unwrap(), 4);
    assert_eq!(*m.get(&5).unwrap(), 3);
    assert_eq!(*m.get(&1).unwrap(), 2);
}

#[test]
fn test_conflict_remove() {
    let mut m = CHashMap::with_capacity(4);
    assert!(m.insert(1, 2).is_none());
    assert_eq!(*m.get(&1).unwrap(), 2);
    assert!(m.insert(5, 3).is_none());
    assert_eq!(*m.get(&1).unwrap(), 2);
    assert_eq!(*m.get(&5).unwrap(), 3);
    assert!(m.insert(9, 4).is_none());
    assert_eq!(*m.get(&1).unwrap(), 2);
    assert_eq!(*m.get(&5).unwrap(), 3);
    assert_eq!(*m.get(&9).unwrap(), 4);
    assert!(m.remove(&1).is_some());
    assert_eq!(*m.get(&9).unwrap(), 4);
    assert_eq!(*m.get(&5).unwrap(), 3);
}

#[test]
fn test_is_empty() {
    let mut m = CHashMap::with_capacity(4);
    assert!(m.insert(1, 2).is_none());
    assert!(!m.is_empty());
    assert!(m.remove(&1).is_some());
    assert!(m.is_empty());
}

#[test]
fn test_pop() {
    let mut m = CHashMap::new();
    m.insert(1, 2);
    assert_eq!(m.remove(&1), Some(2));
    assert_eq!(m.remove(&1), None);
}

#[test]
fn test_keys() {
    let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    let map: CHashMap<_, _> = vec.into_iter().collect();
    let keys: Vec<_> = map.keys().cloned().collect();
    assert_eq!(keys.len(), 3);
    assert!(keys.contains(&1));
    assert!(keys.contains(&2));
    assert!(keys.contains(&3));
}

#[test]
fn test_values() {
    let vec = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    let map: CHashMap<_, _> = vec.into_iter().collect();
    let values: Vec<_> = map.values().cloned().collect();
    assert_eq!(values.len(), 3);
    assert!(values.contains(&'a'));
    assert!(values.contains(&'b'));
    assert!(values.contains(&'c'));
}

#[test]
fn test_find() {
    let mut m = CHashMap::new();
    assert!(m.get(&1).is_none());
    m.insert(1, 2);
    match m.get(&1) {
        None => panic!(),
        Some(v) => assert_eq!(*v, 2)
    }
}

#[test]
fn test_expand() {
    let mut m = CHashMap::new();

    assert_eq!(m.len(), 0);
    assert!(m.is_empty());

    let mut i = 0;
    let old_cap = m.table.capacity();
    while old_cap == m.table.capacity() {
        m.insert(i, i);
        i += 1;
    }

    assert_eq!(m.len(), i);
    assert!(!m.is_empty());
}

#[test]
fn test_behavior_resize_policy() {
    let mut m = CHashMap::new();

    assert_eq!(m.len(), 0);
    assert_eq!(m.table.capacity(), 0);
    assert!(m.is_empty());

    m.insert(0, 0);
    m.remove(&0);
    assert!(m.is_empty());
    let initial_cap = m.table.capacity();
    m.reserve(initial_cap);
    let cap = m.table.capacity();

    assert_eq!(cap, initial_cap * 2);

    let mut i = 0;
    for _ in 0..cap * 3 / 4 {
        m.insert(i, i);
        i += 1;
    }
    // three quarters full

    assert_eq!(m.len(), i);
    assert_eq!(m.table.capacity(), cap);

    for _ in 0..cap / 4 {
        m.insert(i, i);
        i += 1;
    }
    // half full

    let new_cap = m.table.capacity();
    assert_eq!(new_cap, cap * 2);

    for _ in 0..cap / 2 - 1 {
        i -= 1;
        m.remove(&i);
        assert_eq!(m.table.capacity(), new_cap);
    }
    // A little more than one quarter full.
    m.shrink_to_fit();
    assert_eq!(m.table.capacity(), cap);
    // again, a little more than half full
    for _ in 0..cap / 2 - 1 {
        i -= 1;
        m.remove(&i);
    }
    m.shrink_to_fit();

    assert_eq!(m.len(), i);
    assert!(!m.is_empty());
    assert_eq!(m.table.capacity(), initial_cap);
}

#[test]
fn test_reserve_shrink_to_fit() {
    let mut m = CHashMap::new();
    m.insert(0, 0);
    m.remove(&0);
    assert!(m.capacity() >= m.len());
    for i in 0..128 {
        m.insert(i, i);
    }
    m.reserve(256);

    let usable_cap = m.capacity();
    for i in 128..(128 + 256) {
        m.insert(i, i);
        assert_eq!(m.capacity(), usable_cap);
    }

    for i in 100..(128 + 256) {
        assert_eq!(m.remove(&i), Some(i));
    }
    m.shrink_to_fit();

    assert_eq!(m.len(), 100);
    assert!(!m.is_empty());
    assert!(m.capacity() >= m.len());

    for i in 0..100 {
        assert_eq!(m.remove(&i), Some(i));
    }
    m.shrink_to_fit();
    m.insert(0, 0);

    assert_eq!(m.len(), 1);
    assert!(m.capacity() >= m.len());
    assert_eq!(m.remove(&0), Some(0));
}

#[test]
fn test_from_iter() {
    let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

    let map: CHashMap<_, _> = xs.iter().cloned().collect();

    for &(k, v) in &xs {
        assert_eq!(map.get(&k), Some(&v));
    }
}

#[test]
fn test_size_hint() {
    let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

    let map: CHashMap<_, _>  = xs.iter().cloned().collect();

    let mut iter = map.iter();

    for _ in iter.by_ref().take(3) {}

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

#[test]
fn test_iter_len() {
    let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

    let map: CHashMap<_, _>  = xs.iter().cloned().collect();

    let mut iter = map.iter();

    for _ in iter.by_ref().take(3) {}

    assert_eq!(iter.len(), 3);
}

#[test]
fn test_mut_size_hint() {
    let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

    let mut map: CHashMap<_, _>  = xs.iter().cloned().collect();

    let mut iter = map.iter_mut();

    for _ in iter.by_ref().take(3) {}

    assert_eq!(iter.size_hint(), (3, Some(3)));
}

#[test]
fn test_iter_mut_len() {
    let xs = [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (6, 6)];

    let mut map: CHashMap<_, _>  = xs.iter().cloned().collect();

    let mut iter = map.iter_mut();

    for _ in iter.by_ref().take(3) {}

    assert_eq!(iter.len(), 3);
}

#[test]
fn test_capacity_not_less_than_len() {
    let mut a = CHashMap::new();
    let mut item = 0;

    for _ in 0..116 {
        a.insert(item, 0);
        item += 1;
    }

    assert!(a.capacity() > a.len());

    let free = a.capacity() - a.len();
    for _ in 0..free {
        a.insert(item, 0);
        item += 1;
    }

    assert_eq!(a.len(), a.capacity());

    // Insert at capacity should cause allocation.
    a.insert(item, 0);
    assert!(a.capacity() > a.len());
}
