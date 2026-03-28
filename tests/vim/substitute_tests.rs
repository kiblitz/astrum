use astrum::editor::parse_substitute;
use expect_test::{expect, Expect};

fn parse(cmd: &str) -> String {
    match parse_substitute(cmd) {
        Some(sub) => format!(
            "pat={:?} rep={:?} global={} whole_file={} confirm={}",
            sub.pattern, sub.replacement, sub.global, sub.whole_file, sub.confirm
        ),
        None => "None".to_string(),
    }
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

// -- Parsing tests --

#[test]
fn basic_substitute() {
    check(
        &parse("s/foo/bar/"),
        expect![[r#"pat="foo" rep="bar" global=false whole_file=false confirm=false"#]],
    );
}

#[test]
fn global_flag() {
    check(
        &parse("s/foo/bar/g"),
        expect![[r#"pat="foo" rep="bar" global=true whole_file=false confirm=false"#]],
    );
}

#[test]
fn whole_file() {
    check(
        &parse("%s/foo/bar/"),
        expect![[r#"pat="foo" rep="bar" global=false whole_file=true confirm=false"#]],
    );
}

#[test]
fn whole_file_global() {
    check(
        &parse("%s/foo/bar/g"),
        expect![[r#"pat="foo" rep="bar" global=true whole_file=true confirm=false"#]],
    );
}

#[test]
fn custom_delimiter() {
    check(
        &parse("s#foo#bar#g"),
        expect![[r#"pat="foo" rep="bar" global=true whole_file=false confirm=false"#]],
    );
}

#[test]
fn no_trailing_delimiter() {
    check(
        &parse("s/foo/bar"),
        expect![[r#"pat="foo" rep="bar" global=false whole_file=false confirm=false"#]],
    );
}

#[test]
fn empty_replacement() {
    check(
        &parse("s/foo//"),
        expect![[r#"pat="foo" rep="" global=false whole_file=false confirm=false"#]],
    );
}

#[test]
fn empty_pattern_rejected() {
    check(
        &parse("s//bar/"),
        expect!["None"],
    );
}

#[test]
fn confirm_flag() {
    check(
        &parse("s/foo/bar/c"),
        expect!["pat=\"foo\" rep=\"bar\" global=false whole_file=false confirm=true"],
    );
}

#[test]
fn global_confirm_flags() {
    check(
        &parse("s/foo/bar/gc"),
        expect!["pat=\"foo\" rep=\"bar\" global=true whole_file=false confirm=true"],
    );
}

#[test]
fn whole_file_global_confirm() {
    check(
        &parse("%s/foo/bar/gc"),
        expect!["pat=\"foo\" rep=\"bar\" global=true whole_file=true confirm=true"],
    );
}

#[test]
fn not_substitute() {
    check(&parse("set number"), expect!["None"]);
}

#[test]
fn single_field_rejected() {
    check(&parse("s/foo"), expect!["None"]);
}

// -- Replacement logic tests (using Buffer directly) --

use astrum::buffer::TextBuffer;

fn make_buffer(text: &str) -> TextBuffer {
    let mut b = TextBuffer::new_scratch();
    b.rope = ropey::Rope::from_str(text);
    b
}

fn substitute_on(text: &str, cmd: &str) -> String {
    let sub = parse_substitute(cmd).expect("valid substitute command");
    let mut b = make_buffer(text);

    let line_range = if sub.whole_file {
        0..b.rope.len_lines()
    } else {
        b.cursor.line..b.cursor.line + 1
    };

    let lines: Vec<usize> = line_range.collect();
    for &line_idx in lines.iter().rev() {
        if line_idx >= b.rope.len_lines() {
            continue;
        }
        let line_str: String = b.rope.line(line_idx).chars().collect();
        let mut positions = Vec::new();
        let mut search_start = 0;
        while search_start < line_str.len() {
            if let Some(byte_pos) = line_str[search_start..].find(&sub.pattern) {
                let abs = search_start + byte_pos;
                positions.push(abs);
                if !sub.global {
                    break;
                }
                search_start = abs + sub.pattern.len();
            } else {
                break;
            }
        }

        let line_char_start = b.rope.line_to_char(line_idx);
        for &byte_pos in positions.iter().rev() {
            let col_start = line_str[..byte_pos].chars().count();
            let col_end = col_start + sub.pattern.chars().count();
            let char_start = line_char_start + col_start;
            let char_end = line_char_start + col_end;
            b.rope.remove(char_start..char_end);
            b.rope.insert(char_start, &sub.replacement);
        }
    }

    b.rope.to_string()
}

#[test]
fn replace_first_occurrence() {
    check(
        &substitute_on("foo bar foo", "s/foo/baz/"),
        expect!["baz bar foo"],
    );
}

#[test]
fn replace_all_occurrences() {
    check(
        &substitute_on("foo bar foo", "s/foo/baz/g"),
        expect!["baz bar baz"],
    );
}

#[test]
fn replace_whole_file() {
    check(
        &substitute_on("foo\nbar\nfoo", "%s/foo/baz/g"),
        expect!["baz\nbar\nbaz"],
    );
}

#[test]
fn replace_current_line_only() {
    // Cursor is on line 0 by default, so only first line is affected.
    check(
        &substitute_on("foo\nfoo\nfoo", "s/foo/baz/"),
        expect!["baz\nfoo\nfoo"],
    );
}

#[test]
fn replace_with_empty() {
    check(
        &substitute_on("hello world", "s/world//"),
        expect!["hello "],
    );
}

#[test]
fn replace_longer_string() {
    check(
        &substitute_on("ab ab ab", "s/ab/xyz/g"),
        expect!["xyz xyz xyz"],
    );
}

#[test]
fn replace_no_match() {
    check(
        &substitute_on("hello world", "s/xyz/abc/"),
        expect!["hello world"],
    );
}
