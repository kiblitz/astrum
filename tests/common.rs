pub mod import {
    pub use astrum::import::*;
    pub use expect_test::expect;
}

use import::*;

pub fn prefix_tree_of_strings<V: Clone>(
    kv_pairs: Vec<(&str, V)>,
) -> Result<prefix_tree::Map<char, V>> {
    kv_pairs
        .iter()
        .fold(Ok(prefix_tree::Map::new()), |prefix_tree, (key, value)| {
            prefix_tree?.insert(&key.chars().collect(), value.clone())
        })
}
