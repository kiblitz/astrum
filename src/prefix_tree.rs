use crate::import::*;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unable to insert an empty key"))]
    InsertingWithEmptyKeyError,
    #[snafu(display("Insertion would override existing entry: {context}"))]
    OverwritingExistingValueError { context: String },
}

#[derive(Clone, Debug)]
pub struct Map<K: Clone + Debug + Eq + Ord, V: Clone + Debug> {
    map: TreeMap<K, Node<K, V>>,
}

#[derive(Clone, Debug)]
pub enum Node<K: Clone + Debug + Ord, V: Clone + Debug> {
    SubTree(Map<K, V>),
    Value(V),
}

struct InsertionDebugContext<'a, K: Clone + Debug> {
    key_seq: &'a Vec<K>,
    depth: usize,
}

impl<K: Clone + Debug + Ord, V: Clone + Debug> Map<K, V> {
    pub fn new() -> Self {
        Self {
            map: TreeMap::new(),
        }
    }

    pub fn insert(self, key_seq: &Vec<K>, v: V) -> Result<Self> {
        let key_seq_iter = key_seq.iter();
        let result = Self::insert_raw(
            Some(self),
            key_seq_iter,
            v,
            InsertionDebugContext { key_seq, depth: 1 },
        )?;
        match result {
            Node::SubTree(subtree) => Ok(subtree),
            Node::Value(_) => InsertingWithEmptyKeySnafu.fail().context(PrefixTreeSnafu),
        }
    }

    fn insert_raw<'a>(
        curr: Option<Self>,
        mut key_seq_iter: std::slice::Iter<K>,
        v: V,
        mut insertion_debug_context: InsertionDebugContext<'a, K>,
    ) -> Result<Node<K, V>> {
        let k = key_seq_iter.next();
        match k {
            None => {
                let key: &Vec<K> = insertion_debug_context.key_seq;
                if curr.is_some() {
                    (OverwritingExistingValueSnafu {
                        context: format!("(key {key:?})"),
                    })
                    .fail()
                    .context(PrefixTreeSnafu)?;
                }
                Ok(Node::Value(v))
            }
            Some(k) => {
                let curr = curr.unwrap_or_else(|| Self {
                    map: TreeMap::new(),
                });
                let map = match curr.map.get(&k) {
                    None => None,
                    Some(Node::Value(v)) => {
                        let key: &Vec<K> = insertion_debug_context.key_seq;
                        let existing_key: Vec<K> = insertion_debug_context
                            .key_seq
                            .iter()
                            .take(insertion_debug_context.depth)
                            .cloned()
                            .collect();
                        (OverwritingExistingValueSnafu {
                            context: format!("(key {key:?}) (existing_key {existing_key:?}) (existing_value {v:?})"),
                        })
                        .fail()
                        .context(PrefixTreeSnafu)?;
                        None
                    }
                    Some(Node::SubTree(subtree)) => Some(subtree.clone()),
                };
                insertion_debug_context.depth += 1;
                let result = Self::insert_raw(map, key_seq_iter, v, insertion_debug_context)?;
                Ok(Node::SubTree(Self {
                    map: curr.map.insert(k.clone(), result),
                }))
            }
        }
    }

    pub fn enter(&self, key: &K) -> Option<&Node<K, V>> {
        self.map.get(key)
    }
}
