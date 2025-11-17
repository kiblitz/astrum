mod common;
use common::import::*;

fn basic_prefix_tree() -> prefix_tree::Map<char, u8> {
    common::prefix_tree::of_strings(vec![
        ("abc", 0),
        ("abd", 1),
        ("ae", 2),
        ("afg", 3),
        ("h", 4),
    ])
    .unwrap()
}

#[test]
fn test_basic_enter__a() {
    let prefix_tree = basic_prefix_tree();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'a').unwrap();

    let expected = expect![[r#"
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
        }
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}

#[test]
fn test_basic_enter__ab() {
    let prefix_tree = basic_prefix_tree();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'a').unwrap();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'b').unwrap();

    let expected = expect![[r#"
        Map {
            map: {
                'c': Value(
                    0,
                ),
                'd': Value(
                    1,
                ),
            },
        }
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}

#[test]
fn test_basic_enter__abc() {
    let prefix_tree = basic_prefix_tree();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'a').unwrap();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'b').unwrap();
    let v = common::prefix_tree::enter_value(&prefix_tree, &'c').unwrap();

    let expected = expect![[r#"
        0
    "#]];
    expected.assert_debug_eq(&v);
}

#[test]
fn test_basic_enter__ag_none() {
    let prefix_tree = basic_prefix_tree();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'a').unwrap();
    let prefix_tree = common::prefix_tree::enter_subtree(&prefix_tree, &'g');

    let expected = expect![[r#"
        None
    "#]];
    expected.assert_debug_eq(&prefix_tree);
}
