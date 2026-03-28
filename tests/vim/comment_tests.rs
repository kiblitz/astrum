use astrum::buffer::TextBuffer;
use expect_test::{expect, Expect};
use ropey::Rope;

fn buf(text: &str, line: usize, col: usize) -> TextBuffer {
    let mut b = TextBuffer::new_scratch();
    b.rope = Rope::from_str(text);
    b.cursor.line = line;
    b.cursor.col = col;
    b
}

fn state(b: &TextBuffer) -> String {
    let text = b.rope.to_string();
    let display = text.replace('\n', "\\n");
    format!("({},{}) \"{}\"", b.cursor.line, b.cursor.col, display)
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

// -- Commenting (adding prefix) --

#[test]
fn comment_single_line() {
    let mut b = buf("hello world\n", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    check(&state(&b), expect!["(0,0) \"// hello world\\n\""]);
}

#[test]
fn comment_multiple_lines() {
    let mut b = buf("aaa\nbbb\nccc\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    check(&state(&b), expect!["(0,0) \"// aaa\\n// bbb\\n// ccc\\n\""]);
}

#[test]
fn comment_preserves_indentation() {
    let mut b = buf("    foo\n        bar\n    baz\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    // Prefix inserted at min indent (4) for all lines.
    check(
        &state(&b),
        expect!["(0,0) \"    // foo\\n    //     bar\\n    // baz\\n\""],
    );
}

#[test]
fn comment_skips_empty_lines() {
    let mut b = buf("aaa\n\nbbb\n", 0, 0);
    b.toggle_line_comment(0, 2, "#");
    check(&state(&b), expect!["(0,0) \"# aaa\\n\\n# bbb\\n\""]);
}

#[test]
fn comment_mixed_indent_uses_minimum() {
    let mut b = buf("  foo\n    bar\n", 0, 0);
    b.toggle_line_comment(0, 1, "//");
    // Min indent is 2, so prefix inserted at col 2 for both lines.
    check(
        &state(&b),
        expect!["(0,0) \"  // foo\\n  //   bar\\n\""],
    );
}

// -- Uncommenting (removing prefix) --

#[test]
fn uncomment_single_line() {
    let mut b = buf("// hello\n", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    check(&state(&b), expect!["(0,0) \"hello\\n\""]);
}

#[test]
fn uncomment_with_space() {
    let mut b = buf("// hello\n", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    check(&state(&b), expect!["(0,0) \"hello\\n\""]);
}

#[test]
fn uncomment_without_space() {
    let mut b = buf("//hello\n", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    check(&state(&b), expect!["(0,0) \"hello\\n\""]);
}

#[test]
fn uncomment_multiple_lines() {
    let mut b = buf("// aaa\n// bbb\n// ccc\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    check(&state(&b), expect!["(0,0) \"aaa\\nbbb\\nccc\\n\""]);
}

#[test]
fn uncomment_preserves_indentation() {
    let mut b = buf("    // foo\n    // bar\n", 0, 0);
    b.toggle_line_comment(0, 1, "//");
    check(&state(&b), expect!["(0,0) \"    foo\\n    bar\\n\""]);
}

#[test]
fn uncomment_skips_empty_lines() {
    let mut b = buf("# aaa\n\n# bbb\n", 0, 0);
    b.toggle_line_comment(0, 2, "#");
    check(&state(&b), expect!["(0,0) \"aaa\\n\\nbbb\\n\""]);
}

// -- Toggle logic (mixed commented / uncommented → comment all) --

#[test]
fn mixed_lines_comments_all() {
    let mut b = buf("// aaa\nbbb\n// ccc\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    check(
        &state(&b),
        expect!["(0,0) \"// // aaa\\n// bbb\\n// // ccc\\n\""],
    );
}

// -- Different prefixes --

#[test]
fn comment_with_hash() {
    let mut b = buf("print('hello')\n", 0, 0);
    b.toggle_line_comment(0, 0, "#");
    check(&state(&b), expect!["(0,0) \"# print('hello')\\n\""]);
}

#[test]
fn comment_with_double_dash() {
    let mut b = buf("local x = 1\n", 0, 0);
    b.toggle_line_comment(0, 0, "--");
    check(&state(&b), expect!["(0,0) \"-- local x = 1\\n\""]);
}

#[test]
fn comment_with_semicolon() {
    let mut b = buf("(defn foo [])\n", 0, 0);
    b.toggle_line_comment(0, 0, ";");
    check(&state(&b), expect!["(0,0) \"; (defn foo [])\\n\""]);
}

// -- Undo --

#[test]
fn comment_undo_is_atomic() {
    let mut b = buf("aaa\nbbb\nccc\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    // Should be commented now.
    assert!(b.rope.to_string().starts_with("// aaa"));
    // Single undo restores all three lines.
    b.undo();
    check(&state(&b), expect!["(0,0) \"aaa\\nbbb\\nccc\\n\""]);
}

// -- Edge cases --

#[test]
fn comment_single_line_no_trailing_newline() {
    let mut b = buf("hello", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    check(&state(&b), expect!["(0,0) \"// hello\""]);
}

#[test]
fn comment_empty_buffer() {
    let mut b = buf("", 0, 0);
    b.toggle_line_comment(0, 0, "//");
    // Empty line is skipped, so nothing changes.
    check(&state(&b), expect!["(0,0) \"\""]);
}

#[test]
fn comment_only_empty_lines() {
    let mut b = buf("\n\n\n", 0, 0);
    b.toggle_line_comment(0, 2, "//");
    // All empty lines → all "commented" → uncomment path, but all skip → no change.
    check(&state(&b), expect!["(0,0) \"\\n\\n\\n\""]);
}

#[test]
fn comment_range_clamped_to_buffer() {
    let mut b = buf("aaa\nbbb\n", 0, 0);
    // Range extends past end of buffer.
    b.toggle_line_comment(0, 100, "//");
    check(&state(&b), expect!["(0,0) \"// aaa\\n// bbb\\n\""]);
}

#[test]
fn uncomment_then_recomment() {
    let mut b = buf("// hello\n// world\n", 0, 0);
    b.toggle_line_comment(0, 1, "//");
    check(&state(&b), expect!["(0,0) \"hello\\nworld\\n\""]);
    b.toggle_line_comment(0, 1, "//");
    check(&state(&b), expect!["(0,0) \"// hello\\n// world\\n\""]);
}

#[test]
fn comment_multibyte() {
    let mut b = buf("  café\n  naïve\n", 0, 0);
    b.toggle_line_comment(0, 1, "//");
    check(
        &state(&b),
        expect!["(0,0) \"  // café\\n  // naïve\\n\""],
    );
}

#[test]
fn uncomment_multibyte() {
    let mut b = buf("  // café\n  // naïve\n", 0, 0);
    b.toggle_line_comment(0, 1, "//");
    check(&state(&b), expect!["(0,0) \"  café\\n  naïve\\n\""]);
}

// -- Block comments --

#[test]
fn block_comment_wrap() {
    let mut b = buf("hello world\n", 0, 0);
    // Select "hello" (chars 0..5)
    b.toggle_block_comment(0, 5, "/*", "*/");
    check(&state(&b), expect!["(0,0) \"/*hello*/ world\\n\""]);
}

#[test]
fn block_comment_unwrap() {
    let mut b = buf("/*hello*/ world\n", 0, 0);
    // Select "/*hello*/" (chars 0..9)
    b.toggle_block_comment(0, 9, "/*", "*/");
    check(&state(&b), expect!["(0,0) \"hello world\\n\""]);
}

#[test]
fn block_comment_multiline() {
    let mut b = buf("aaa\nbbb\nccc\n", 0, 0);
    // Select all of "aaa\nbbb\n" (chars 0..8)
    b.toggle_block_comment(0, 8, "/*", "*/");
    check(&state(&b), expect!["(0,0) \"/*aaa\\nbbb\\n*/ccc\\n\""]);
}

#[test]
fn block_comment_unwrap_multiline() {
    let mut b = buf("/*aaa\nbbb\n*/ccc\n", 0, 0);
    b.toggle_block_comment(0, 12, "/*", "*/");
    check(&state(&b), expect!["(0,0) \"aaa\\nbbb\\nccc\\n\""]);
}

#[test]
fn block_comment_html() {
    let mut b = buf("<div>hello</div>\n", 0, 0);
    b.toggle_block_comment(0, 16, "<!--", "-->");
    check(&state(&b), expect!["(0,0) \"<!--<div>hello</div>-->\\n\""]);
}

#[test]
fn block_comment_unwrap_html() {
    let mut b = buf("<!--<div>hello</div>-->\n", 0, 0);
    b.toggle_block_comment(0, 23, "<!--", "-->");
    check(&state(&b), expect!["(0,0) \"<div>hello</div>\\n\""]);
}

#[test]
fn block_comment_undo_is_atomic() {
    let mut b = buf("hello world\n", 0, 0);
    b.toggle_block_comment(0, 5, "/*", "*/");
    assert!(b.rope.to_string().starts_with("/*hello*/"));
    b.undo();
    check(&state(&b), expect!["(0,0) \"hello world\\n\""]);
}

#[test]
fn block_comment_empty_range() {
    let mut b = buf("hello\n", 0, 0);
    b.toggle_block_comment(0, 0, "/*", "*/");
    // Empty range, nothing happens.
    check(&state(&b), expect!["(0,0) \"hello\\n\""]);
}

#[test]
fn block_comment_with_whitespace() {
    let mut b = buf("/* hello */\n", 0, 0);
    // Selection includes the block comment with inner spaces.
    b.toggle_block_comment(0, 11, "/*", "*/");
    check(&state(&b), expect!["(0,0) \" hello \\n\""]);
}

#[test]
fn block_comment_lua() {
    let mut b = buf("local x = 1\n", 0, 0);
    b.toggle_block_comment(0, 11, "--[[", "]]");
    check(&state(&b), expect!["(0,0) \"--[[local x = 1]]\\n\""]);
}

#[test]
fn block_comment_multibyte_chars() {
    let mut b = buf("café naïve\n", 0, 0);
    // Select "café" (4 chars)
    b.toggle_block_comment(0, 4, "/*", "*/");
    check(&state(&b), expect!["(0,0) \"/*café*/ naïve\\n\""]);
}
