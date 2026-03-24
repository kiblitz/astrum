use astrum::editor::find_all_matches_in_rope;
use expect_test::{expect, Expect};
use ropey::Rope;

fn matches(text: &str, pattern: &str) -> String {
    let rope = Rope::from_str(text);
    let results = find_all_matches_in_rope(&rope, pattern);
    format!("{:?}", results)
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

#[test]
fn overlapping_parens() {
    check(
        &matches("(((((", "(("),
        expect!["[(0, 0, 2), (0, 1, 3), (0, 2, 4), (0, 3, 5)]"],
    );
}

#[test]
fn non_overlapping_simple() {
    check(
        &matches("hello hello hello", "hello"),
        expect!["[(0, 0, 5), (0, 6, 11), (0, 12, 17)]"],
    );
}

#[test]
fn overlapping_aa() {
    check(
        &matches("aaaa", "aa"),
        expect!["[(0, 0, 2), (0, 1, 3), (0, 2, 4)]"],
    );
}

#[test]
fn no_match() {
    check(
        &matches("hello world", "xyz"),
        expect!["[]"],
    );
}

#[test]
fn multiline() {
    check(
        &matches("foo\nbar\nfoo", "foo"),
        expect!["[(0, 0, 3), (2, 0, 3)]"],
    );
}

#[test]
fn single_char_pattern() {
    check(
        &matches("abcabc", "a"),
        expect!["[(0, 0, 1), (0, 3, 4)]"],
    );
}

#[test]
fn overlapping_aba() {
    check(
        &matches("ababa", "aba"),
        expect!["[(0, 0, 3), (0, 2, 5)]"],
    );
}

#[test]
fn empty_lines() {
    check(
        &matches("\n\nhello\n", "hello"),
        expect!["[(2, 0, 5)]"],
    );
}
