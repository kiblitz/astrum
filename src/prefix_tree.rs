use crate::import::*;

type StdResult<T> = crate::error::Result<T>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to insert an empty key"))]
    InsertingWithEmptyKeyError,
}

#[derive(Debug, Clone)]
pub struct Map<K: Clone + Eq + Hash + Ord, V: Clone> {
    map: TreeMap<K, Result<K, V>>,
}

#[derive(Debug, Clone)]
pub enum Result<K: Clone + Eq + Hash + Ord, V: Clone> {
    SubTree(Map<K, V>),
    Value(V),
}

impl<K: Clone + Eq + Hash + Ord, V: Clone> Map<K, V> {
    pub fn new() -> Self {
        Self {
            map: TreeMap::new(),
        }
    }

    pub fn insert(self, key_seq: Vec<K>, v: V) -> StdResult<Self> {
        let key_seq_iter = key_seq.iter();
        let result = Self::insert_raw(Some(self), key_seq_iter, v);
        match result {
            Result::SubTree(subtree) => Ok(subtree),
            Result::Value(_) => InsertingWithEmptyKeySnafu.fail().context(PrefixTreeSnafu),
        }
    }

    fn insert_raw(curr: Option<Self>, mut key_seq_iter: std::slice::Iter<K>, v: V) -> Result<K, V> {
        let curr = curr.unwrap_or_else(|| Self {
            map: TreeMap::new(),
        });
        let k = key_seq_iter.next();
        match k {
            None => Result::Value(v),
            Some(k) => {
                let map = match curr.map.get(&k) {
                    None | Some(Result::Value(_)) => None,
                    Some(Result::SubTree(subtree)) => Some(subtree.clone()),
                };
                let result = Self::insert_raw(map, key_seq_iter, v);
                Result::SubTree(Self {
                    map: curr.map.insert(k.clone(), result),
                })
            }
        }
    }
}
