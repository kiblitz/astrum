pub mod import {
    pub use astrum::import::*;
    pub use expect_test::expect;
}

pub mod prefix_tree {
    pub use astrum::import::*;

    pub fn of_strings<V: Clone + Debug>(
        kv_pairs: Vec<(&str, V)>,
    ) -> Result<prefix_tree::Map<char, V>> {
        kv_pairs
            .iter()
            .fold(Ok(prefix_tree::Map::new()), |prefix_tree, (key, value)| {
                prefix_tree?.insert(&key.chars().collect(), value.clone())
            })
    }

    pub fn enter_subtree<'a, K: Clone + Debug + Ord, V: Clone + Debug>(
        prefix_tree: &'a prefix_tree::Map<K, V>,
        key: &K,
    ) -> Option<&'a prefix_tree::Map<K, V>> {
        match prefix_tree.enter(key)? {
            prefix_tree::Node::SubTree(subtree) => Some(subtree),
            _ => panic!("tried to enter non-subtree"),
        }
    }

    pub fn enter_value<'a, K: Clone + Debug + Ord, V: Clone + Debug>(
        prefix_tree: &'a prefix_tree::Map<K, V>,
        key: &K,
    ) -> Option<&'a V> {
        match prefix_tree.enter(key)? {
            prefix_tree::Node::Value(v) => Some(v),
            _ => panic!("tried to enter non-value"),
        }
    }
}
