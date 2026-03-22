use astrum::buffer::Buffer;
use expect_test::{expect, Expect};
use ropey::Rope;

/// Create a buffer from text with cursor at (line, col).
fn buf(text: &str, line: usize, col: usize) -> Buffer {
    let mut b = Buffer::new_scratch();
    b.rope = Rope::from_str(text);
    b.cursor.line = line;
    b.cursor.col = col;
    b
}

/// Snapshot: cursor position + buffer text (newlines escaped).
fn state(b: &Buffer) -> String {
    let text = b.rope.to_string();
    let display = text.replace('\n', "\\n");
    format!("cursor=({},{}) text=\"{}\"", b.cursor.line, b.cursor.col, display)
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

// -- line_count --

#[test]
fn line_count_no_trailing_newline() {
    let b = buf("hello", 0, 0);
    check(&b.line_count().to_string(), expect!["1"]);
}

#[test]
fn line_count_with_trailing_newline() {
    let b = buf("hello\n", 0, 0);
    check(&b.line_count().to_string(), expect!["1"]);
}

#[test]
fn line_count_multiline() {
    let b = buf("hello\nworld\n", 0, 0);
    check(&b.line_count().to_string(), expect!["2"]);
}

#[test]
fn line_count_multiline_no_trailing() {
    let b = buf("hello\nworld", 0, 0);
    check(&b.line_count().to_string(), expect!["2"]);
}

// -- word forward --

#[test]
fn word_forward_simple() {
    let mut b = buf("hello world", 0, 0);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,6) text="hello world""#]]);
}

#[test]
fn word_forward_punctuation_boundary() {
    let mut b = buf("main()", 0, 0);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,4) text="main()""#]]);
}

#[test]
fn word_forward_space_then_punctuation() {
    let mut b = buf("main ()", 0, 0);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,5) text="main ()""#]]);
}

#[test]
fn word_forward_from_punctuation() {
    let mut b = buf("foo.bar", 0, 3);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,4) text="foo.bar""#]]);
}

#[test]
fn word_forward_multiple_punctuation() {
    let mut b = buf("a::b", 0, 0);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,1) text="a::b""#]]);
}

#[test]
fn word_forward_across_line() {
    let mut b = buf("end\nstart", 0, 0);
    b.move_word_forward();
    check(&state(&b), expect![[r#"cursor=(1,0) text="end\nstart""#]]);
}

// -- word backward --

#[test]
fn word_backward_simple() {
    let mut b = buf("hello world", 0, 6);
    b.move_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,0) text="hello world""#]]);
}

#[test]
fn word_backward_punctuation() {
    let mut b = buf("foo.bar", 0, 4);
    b.move_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,3) text="foo.bar""#]]);
}

#[test]
fn word_backward_from_word_start() {
    let mut b = buf("hello world", 0, 6);
    b.move_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,0) text="hello world""#]]);
}

#[test]
fn word_backward_skips_punctuation_class() {
    let mut b = buf("a::b", 0, 3);
    b.move_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,1) text="a::b""#]]);
}

// -- word end --

#[test]
fn word_end_simple() {
    let mut b = buf("hello world", 0, 0);
    b.move_word_end();
    check(&state(&b), expect![[r#"cursor=(0,4) text="hello world""#]]);
}

#[test]
fn word_end_at_end_of_word() {
    let mut b = buf("hello world", 0, 4);
    b.move_word_end();
    check(&state(&b), expect![[r#"cursor=(0,10) text="hello world""#]]);
}

#[test]
fn word_end_punctuation() {
    let mut b = buf("foo..bar", 0, 0);
    b.move_word_end();
    check(&state(&b), expect![[r#"cursor=(0,2) text="foo..bar""#]]);
}

#[test]
fn word_end_from_punctuation() {
    let mut b = buf("foo..bar", 0, 3);
    b.move_word_end();
    check(&state(&b), expect![[r#"cursor=(0,4) text="foo..bar""#]]);
}

// -- cursor clamping --

#[test]
fn clamp_cursor_normal_mode() {
    let mut b = buf("hello", 0, 10);
    b.clamp_cursor();
    check(&state(&b), expect![[r#"cursor=(0,4) text="hello""#]]);
}

#[test]
fn clamp_cursor_empty_line() {
    let mut b = buf("hello\n\nworld", 1, 5);
    b.clamp_cursor();
    check(&state(&b), expect![[r#"cursor=(1,0) text="hello\n\nworld""#]]);
}

// -- linewise_range --

#[test]
fn linewise_range_single_line() {
    let b = buf("hello\nworld\n", 0, 0);
    let (start, end) = b.linewise_range(0, 0);
    check(&format!("{}..{}", start, end), expect!["0..6"]);
}

#[test]
fn linewise_range_multiple_lines() {
    let b = buf("aaa\nbbb\nccc\n", 0, 0);
    let (start, end) = b.linewise_range(0, 1);
    check(&format!("{}..{}", start, end), expect!["0..8"]);
}

#[test]
fn linewise_range_last_line_no_newline() {
    let b = buf("aaa\nbbb", 0, 0);
    let (start, end) = b.linewise_range(1, 1);
    check(&format!("{}..{}", start, end), expect!["4..7"]);
}

// -- delete_char_range --

#[test]
fn delete_char_range_middle() {
    let mut b = buf("hello world", 0, 0);
    b.delete_char_range(5, 11);
    check(&state(&b), expect![[r#"cursor=(0,4) text="hello""#]]);
}

// -- insert and undo --

#[test]
fn insert_char_and_undo() {
    let mut b = buf("hllo", 0, 1);
    b.insert_char('e');
    check(&state(&b), expect![[r#"cursor=(0,2) text="hello""#]]);
    b.undo();
    check(&state(&b), expect![[r#"cursor=(0,1) text="hllo""#]]);
}

// -- insert_line_below --

#[test]
fn insert_line_below_middle() {
    let mut b = buf("aaa\nbbb\n", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"cursor=(1,0) text="aaa\n\nbbb\n""#]]);
}

#[test]
fn insert_line_below_last_line_with_newline() {
    let mut b = buf("aaa\n", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"cursor=(1,0) text="aaa\n\n""#]]);
}

#[test]
fn insert_line_below_last_line_no_newline() {
    let mut b = buf("aaa", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"cursor=(1,0) text="aaa\n\n""#]]);
}

// -- delete_line --

#[test]
fn delete_line_middle() {
    let mut b = buf("aaa\nbbb\nccc\n", 1, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"cursor=(1,0) text="aaa\nccc\n""#]]);
}

#[test]
fn delete_line_last() {
    let mut b = buf("aaa\nbbb\n", 1, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"cursor=(0,0) text="aaa\n""#]]);
}

#[test]
fn delete_line_only_line() {
    let mut b = buf("hello", 0, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"cursor=(0,0) text="""#]]);
}

// -- move_to_line_end --

#[test]
fn move_to_line_end_normal() {
    let mut b = buf("hello", 0, 0);
    b.move_to_line_end();
    check(&state(&b), expect![[r#"cursor=(0,4) text="hello""#]]);
}

#[test]
fn move_to_line_end_with_newline() {
    let mut b = buf("hello\nworld", 0, 0);
    b.move_to_line_end();
    check(&state(&b), expect![[r#"cursor=(0,4) text="hello\nworld""#]]);
}

// -- char_idx_at --

#[test]
fn char_idx_at_basic() {
    let b = buf("hello\nworld", 0, 0);
    check(&b.char_idx_at(0, 3).to_string(), expect!["3"]);
    check(&b.char_idx_at(1, 2).to_string(), expect!["8"]);
}

// -- move_right clamping --

#[test]
fn move_right_clamps_at_last_char() {
    let mut b = buf("abc", 0, 2);
    b.move_right();
    check(&state(&b), expect![[r#"cursor=(0,2) text="abc""#]]);
}

// -- big word motions --

#[test]
fn big_word_forward() {
    let mut b = buf("foo.bar baz", 0, 0);
    b.move_big_word_forward();
    check(&state(&b), expect![[r#"cursor=(0,8) text="foo.bar baz""#]]);
}

#[test]
fn big_word_backward() {
    let mut b = buf("foo.bar baz", 0, 8);
    b.move_big_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,0) text="foo.bar baz""#]]);
}

// -- delete word backward --

#[test]
fn delete_word_backward_simple() {
    let mut b = buf("hello world", 0, 11);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,6) text="hello ""#]]);
}

#[test]
fn delete_word_backward_mid_word() {
    let mut b = buf("hello world", 0, 8);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,6) text="hello rld""#]]);
}

#[test]
fn delete_word_backward_at_start() {
    let mut b = buf("hello world", 0, 0);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,0) text="hello world""#]]);
}

#[test]
fn delete_word_backward_across_line() {
    let mut b = buf("hello\nworld", 1, 0);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,0) text="world""#]]);
}

#[test]
fn delete_word_backward_punctuation() {
    let mut b = buf("foo.bar", 0, 7);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,4) text="foo.""#]]);
}

#[test]
fn delete_word_backward_undo_restores_cursor() {
    let mut b = buf("hello world", 0, 11);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"cursor=(0,6) text="hello ""#]]);
    b.undo();
    check(&state(&b), expect![[r#"cursor=(0,11) text="hello world""#]]);
}
