mod common;
use common::import::*;

#[test]
fn test_prefix_tree() {
    let prefix_tree = common::prefix_tree_of_strings(vec![
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
