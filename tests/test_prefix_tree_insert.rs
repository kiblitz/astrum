mod common;
use common::import::*;

#[test]
fn test_basic_construction() {
    let prefix_tree = common::prefix_tree::of_strings(vec![
        ("abc", 0),
        ("abd", 1),
        ("ae", 2),
        ("afg", 3),
        ("h", 4),
    ]);

    let expected = expect![[r#"
        Ok(
            Map {
                map: {
                    'a': SubTree(
                        Map {
                            map: {
                                'b': SubTree(
                                    Map {
                                        map: {
                                            'c': Value(
                                                0,
                                            ),
                                            'd': Value(
                                                1,
                                            ),
                                        },
                                    },
                                ),
                                'e': Value(
                                    2,
                                ),
                                'f': SubTree(
                                    Map {
                                        map: {
                                            'g': Value(
                                                3,
                                            ),
                                        },
                                    },
                                ),
                            },
                        },
                    ),
                    'h': Value(
                        4,
                    ),
                },
            },
        )
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}

#[test]
fn test_overwrite_with_prefix() {
    let prefix_tree = common::prefix_tree::of_strings(vec![("abc", 0), ("a", 1)]);

    let expected = expect![[r#"
        Err(
            PrefixTreeError {
                source: OverwritingExistingValueError {
                    context: "(key ['a'])",
                },
            },
        )
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}

#[test]
fn test_overwrite_prefix() {
    let prefix_tree = common::prefix_tree::of_strings(vec![("a", 0), ("abc", 1)]);

    let expected = expect![[r#"
        Err(
            PrefixTreeError {
                source: OverwritingExistingValueError {
                    context: "(key ['a', 'b', 'c']) (existing_key ['a']) (existing_value 0)",
                },
            },
        )
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}

#[test]
fn test_overwrite_prefix2() {
    let prefix_tree =
        common::prefix_tree::of_strings(vec![("hello", 0), ("help", 1), ("hello world", 2)]);

    let expected = expect![[r#"
        Err(
            PrefixTreeError {
                source: OverwritingExistingValueError {
                    context: "(key ['h', 'e', 'l', 'l', 'o', ' ', 'w', 'o', 'r', 'l', 'd']) (existing_key ['h', 'e', 'l', 'l', 'o']) (existing_value 0)",
                },
            },
        )
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}
