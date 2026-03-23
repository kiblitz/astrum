use astrum::buffer::Buffer;
use expect_test::{expect, Expect};
use ropey::Rope;

fn buf(text: &str, line: usize, col: usize) -> Buffer {
    let mut b = Buffer::new_scratch();
    b.rope = Rope::from_str(text);
    b.cursor.line = line;
    b.cursor.col = col;
    b
}

fn state(b: &Buffer) -> String {
    let text = b.rope.to_string();
    let display = text.replace('\n', "\\n");
    format!("({},{}) \"{}\"", b.cursor.line, b.cursor.col, display)
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

// -- insert_char --

#[test]
fn insert_char_middle() {
    let mut b = buf("hllo", 0, 1);
    b.insert_char('e');
    check(&state(&b), expect![[r#"(0,2) "hello""#]]);
}

#[test]
fn insert_char_at_start() {
    let mut b = buf("ello", 0, 0);
    b.insert_char('h');
    check(&state(&b), expect![[r#"(0,1) "hello""#]]);
}

#[test]
fn insert_char_at_end() {
    let mut b = buf("hell", 0, 4);
    b.insert_char('o');
    check(&state(&b), expect![[r#"(0,5) "hello""#]]);
}

#[test]
fn insert_char_empty_buffer() {
    let mut b = buf("", 0, 0);
    b.insert_char('x');
    check(&state(&b), expect![[r#"(0,1) "x""#]]);
}

#[test]
fn insert_char_second_line() {
    let mut b = buf("aaa\nbbb", 1, 1);
    b.insert_char('X');
    check(&state(&b), expect![[r#"(1,2) "aaa\nbXbb""#]]);
}

// -- insert_newline --

#[test]
fn insert_newline_middle() {
    let mut b = buf("helloworld", 0, 5);
    b.insert_newline();
    check(&state(&b), expect![[r#"(1,0) "hello\nworld""#]]);
}

#[test]
fn insert_newline_at_start() {
    let mut b = buf("hello", 0, 0);
    b.insert_newline();
    check(&state(&b), expect![[r#"(1,0) "\nhello""#]]);
}

#[test]
fn insert_newline_at_end() {
    let mut b = buf("hello", 0, 5);
    b.insert_newline();
    check(&state(&b), expect![[r#"(1,0) "hello\n""#]]);
}

#[test]
fn insert_newline_empty() {
    let mut b = buf("", 0, 0);
    b.insert_newline();
    check(&state(&b), expect![[r#"(1,0) "\n""#]]);
}

// -- delete_char_backward --

#[test]
fn delete_char_backward_middle() {
    let mut b = buf("hello", 0, 3);
    b.delete_char_backward();
    check(&state(&b), expect![[r#"(0,2) "helo""#]]);
}

#[test]
fn delete_char_backward_at_start_noop() {
    let mut b = buf("hello", 0, 0);
    b.delete_char_backward();
    check(&state(&b), expect![[r#"(0,0) "hello""#]]);
}

#[test]
fn delete_char_backward_joins_lines() {
    let mut b = buf("hello\nworld", 1, 0);
    b.delete_char_backward();
    check(&state(&b), expect![[r#"(0,10) "helloworld""#]]);
}

#[test]
fn delete_char_backward_single_char() {
    let mut b = buf("x", 0, 1);
    b.delete_char_backward();
    check(&state(&b), expect![[r#"(0,0) """#]]);
}

// -- delete_char_forward --

#[test]
fn delete_char_forward_middle() {
    let mut b = buf("hello", 0, 2);
    b.delete_char_forward();
    check(&state(&b), expect![[r#"(0,2) "helo""#]]);
}

#[test]
fn delete_char_forward_at_end_noop() {
    let mut b = buf("hello", 0, 5);
    b.delete_char_forward();
    check(&state(&b), expect![[r#"(0,5) "hello""#]]);
}

#[test]
fn delete_char_forward_joins_lines() {
    let mut b = buf("hello\nworld", 0, 5);
    b.delete_char_forward();
    check(&state(&b), expect![[r#"(0,5) "helloworld""#]]);
}

// -- delete_word_backward --

#[test]
fn delete_word_backward_simple() {
    let mut b = buf("hello world", 0, 11);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,6) "hello ""#]]);
}

#[test]
fn delete_word_backward_mid_word() {
    let mut b = buf("hello world", 0, 8);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,6) "hello rld""#]]);
}

#[test]
fn delete_word_backward_at_start() {
    let mut b = buf("hello world", 0, 0);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,0) "hello world""#]]);
}

#[test]
fn delete_word_backward_across_line() {
    let mut b = buf("hello\nworld", 1, 0);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn delete_word_backward_punctuation() {
    let mut b = buf("foo.bar", 0, 7);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,4) "foo.""#]]);
}

#[test]
fn delete_word_backward_undo_restores_cursor() {
    let mut b = buf("hello world", 0, 11);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,6) "hello ""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,11) "hello world""#]]);
}

#[test]
fn delete_word_backward_multiple_spaces() {
    let mut b = buf("hello    world", 0, 9);
    b.delete_word_backward();
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

// -- delete_line --

#[test]
fn delete_line_middle() {
    let mut b = buf("aaa\nbbb\nccc\n", 1, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"(1,0) "aaa\nccc\n""#]]);
}

#[test]
fn delete_line_last() {
    let mut b = buf("aaa\nbbb\n", 1, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"(0,0) "aaa\n""#]]);
}

#[test]
fn delete_line_only_line() {
    let mut b = buf("hello", 0, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"(0,0) """#]]);
}

#[test]
fn delete_line_first_of_many() {
    let mut b = buf("aaa\nbbb\nccc", 0, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"(0,0) "bbb\nccc""#]]);
}

// -- delete_char_range --

#[test]
fn delete_char_range_middle() {
    let mut b = buf("hello world", 0, 0);
    b.delete_char_range(5, 11);
    check(&state(&b), expect![[r#"(0,4) "hello""#]]);
}

#[test]
fn delete_char_range_start() {
    let mut b = buf("hello world", 0, 0);
    b.delete_char_range(0, 6);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn delete_char_range_across_lines() {
    let mut b = buf("hello\nworld", 0, 0);
    b.delete_char_range(3, 9);
    check(&state(&b), expect![[r#"(0,3) "helld""#]]);
}

// -- text_in_char_range --

#[test]
fn text_in_char_range_basic() {
    let b = buf("hello world", 0, 0);
    check(&b.text_in_char_range(6, 11), expect!["world"]);
}

#[test]
fn text_in_char_range_across_lines() {
    let b = buf("hello\nworld", 0, 0);
    check(&b.text_in_char_range(3, 9), expect!["lo\nwor"]);
}

#[test]
fn text_in_char_range_empty_range() {
    let b = buf("hello", 0, 0);
    check(&b.text_in_char_range(3, 3), expect![""]);
}

#[test]
fn text_in_char_range_empty_line() {
    let b = buf("hello\n\nworld", 0, 0);
    // Empty line (line 1) is just "\n" — linewise range includes it.
    let (start, end) = b.linewise_range(1, 1);
    let text = b.text_in_char_range(start, end);
    // Trim reveals no visible content.
    check(&text.trim().is_empty().to_string(), expect!["true"]);
}

#[test]
fn delete_char_range_empty_is_noop() {
    let mut b = buf("hello", 0, 2);
    let result = b.delete_char_range(2, 2);
    assert!(result.is_none());
    check(&state(&b), expect![[r#"(0,2) "hello""#]]);
}

// -- insert_line_below --

#[test]
fn insert_line_below_middle() {
    let mut b = buf("aaa\nbbb\n", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"(1,0) "aaa\n\nbbb\n""#]]);
}

#[test]
fn insert_line_below_last_line_with_newline() {
    let mut b = buf("aaa\n", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"(1,0) "aaa\n\n""#]]);
}

#[test]
fn insert_line_below_last_line_no_newline() {
    let mut b = buf("aaa", 0, 0);
    b.insert_line_below();
    check(&state(&b), expect![[r#"(1,0) "aaa\n\n""#]]);
}

// -- insert_line_above --

#[test]
fn insert_line_above_middle() {
    let mut b = buf("aaa\nbbb\n", 1, 0);
    b.insert_line_above();
    check(&state(&b), expect![[r#"(1,0) "aaa\n\nbbb\n""#]]);
}

#[test]
fn insert_line_above_first_line() {
    let mut b = buf("aaa\nbbb", 0, 0);
    b.insert_line_above();
    check(&state(&b), expect![[r#"(0,0) "\naaa\nbbb""#]]);
}

// -- undo/redo --

#[test]
fn undo_insert_char() {
    let mut b = buf("hllo", 0, 1);
    b.insert_char('e');
    check(&state(&b), expect![[r#"(0,2) "hello""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,1) "hllo""#]]);
}

#[test]
fn redo_after_undo() {
    let mut b = buf("hllo", 0, 1);
    b.insert_char('e');
    b.undo();
    b.redo();
    check(&state(&b), expect![[r#"(0,2) "hello""#]]);
}

#[test]
fn redo_cleared_after_new_edit() {
    let mut b = buf("abc", 0, 1);
    b.insert_char('X');
    b.undo();
    // New edit should clear redo stack
    b.insert_char('Y');
    b.redo(); // should be noop
    check(&state(&b), expect![[r#"(0,2) "aYbc""#]]);
}

#[test]
fn undo_delete_line() {
    let mut b = buf("aaa\nbbb", 0, 0);
    b.delete_line();
    check(&state(&b), expect![[r#"(0,0) "bbb""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,0) "aaa\nbbb""#]]);
}

#[test]
fn multiple_undo() {
    let mut b = buf("abc", 0, 3);
    b.insert_char('1');
    b.insert_char('2');
    check(&state(&b), expect![[r#"(0,5) "abc12""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,4) "abc1""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,3) "abc""#]]);
}

#[test]
fn undo_at_empty_stack() {
    let mut b = buf("hello", 0, 0);
    b.undo(); // noop
    check(&state(&b), expect![[r#"(0,0) "hello""#]]);
}

// -- paste --

#[test]
fn paste_after_single_line() {
    let mut b = buf("hello", 0, 2);
    b.paste_after("XY");
    check(&state(&b), expect![[r#"(0,4) "helXYlo""#]]);
}

#[test]
fn paste_after_linewise() {
    let mut b = buf("aaa\nbbb", 0, 0);
    b.paste_after("ccc\n");
    check(&state(&b), expect![[r#"(1,0) "aaa\nccc\nbbb""#]]);
}

#[test]
fn paste_before_single_line() {
    let mut b = buf("hello", 0, 2);
    b.paste_before("XY");
    check(&state(&b), expect![[r#"(0,2) "heXYllo""#]]);
}

#[test]
fn paste_before_linewise() {
    let mut b = buf("aaa\nbbb", 1, 0);
    b.paste_before("ccc\n");
    check(&state(&b), expect![[r#"(1,0) "aaa\nccc\nbbb""#]]);
}

#[test]
fn paste_after_empty_string() {
    let mut b = buf("hello", 0, 2);
    b.paste_after("");
    check(&state(&b), expect![[r#"(0,2) "hello""#]]);
}

// -- line_count --

#[test]
fn line_count_single() {
    let b = buf("hello", 0, 0);
    check(&b.line_count().to_string(), expect!["1"]);
}

#[test]
fn line_count_trailing_newline() {
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

#[test]
fn line_count_empty() {
    let b = buf("", 0, 0);
    check(&b.line_count().to_string(), expect!["1"]);
}

// -- modified flag --

#[test]
fn modified_after_insert() {
    let mut b = buf("hello", 0, 0);
    assert!(!b.modified);
    b.insert_char('x');
    assert!(b.modified);
}

#[test]
fn modified_after_delete() {
    let mut b = buf("hello", 0, 1);
    b.delete_char_backward();
    assert!(b.modified);
}

// -- paste with multi-byte characters --

#[test]
fn paste_after_multibyte_inline() {
    let mut b = buf("ab", 0, 0);
    b.paste_after("你好");
    // "你好" is 2 chars, inserted after col 0 → "a你好b"
    // Cursor should be at col 2 (end of inserted text: insert_col=1, +2 chars -1 = 2)
    check(&state(&b), expect![[r#"(0,2) "a你好b""#]]);
}

#[test]
fn paste_after_multibyte_with_newline() {
    let mut b = buf("ab", 0, 0);
    b.paste_after("你\n好");
    // Inserted after col 0: "a你\n好b"
    // Cursor should land on line 1, col 0 (one char "好" after last \n, minus 1)
    check(&state(&b), expect![[r#"(1,0) "a你\n好b""#]]);
}

#[test]
fn paste_before_multibyte_inline() {
    let mut b = buf("ab", 0, 1);
    b.paste_before("你好");
    check(&state(&b), expect![[r#"(0,1) "a你好b""#]]);
}

// -- cc (linewise change) undo atomicity --

#[test]
fn linewise_change_single_undo() {
    // Simulate what cc does: delete lines and insert blank line.
    // This tests that the buffer operations can be composed atomically.
    let mut b = buf("aaa\nbbb\nccc\n", 0, 0);
    // Atomic: save_undo once, delete line range, insert blank line.
    b.save_undo();
    let (start, end) = b.linewise_range(0, 1); // lines 0-1
    b.rope.remove(start..end);
    let pos = start.min(b.rope.len_chars());
    b.rope.insert_char(pos, '\n');
    b.cursor.line = b.rope.char_to_line(pos);
    b.cursor.col = 0;
    check(&state(&b), expect![[r#"(0,0) "\nccc\n""#]]);
    // Single undo should restore everything.
    b.undo();
    check(&state(&b), expect![[r#"(0,0) "aaa\nbbb\nccc\n""#]]);
}

#[test]
fn linewise_change_single_line() {
    let mut b = buf("hello\nworld\n", 0, 0);
    b.save_undo();
    let (start, end) = b.linewise_range(0, 0);
    b.rope.remove(start..end);
    let pos = start.min(b.rope.len_chars());
    b.rope.insert_char(pos, '\n');
    b.cursor.line = b.rope.char_to_line(pos);
    b.cursor.col = 0;
    check(&state(&b), expect![[r#"(0,0) "\nworld\n""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,0) "hello\nworld\n""#]]);
}

// -- paste linewise --

#[test]
fn paste_after_linewise_last_line_no_newline() {
    let mut b = buf("aaa", 0, 0);
    b.paste_after("bbb\n");
    check(&state(&b), expect![[r#"(1,0) "aaa\nbbb\n""#]]);
}

#[test]
fn paste_before_linewise_multibyte() {
    let mut b = buf("bbb\n", 0, 0);
    b.paste_before("aaa\n");
    check(&state(&b), expect![[r#"(0,0) "aaa\nbbb\n""#]]);
}

// -- char_idx_at with multi-byte --

#[test]
fn char_idx_at_multibyte() {
    let b = buf("你好世界", 0, 0);
    // Each char is 1 char index, not 3 bytes
    check(&b.char_idx_at(0, 2).to_string(), expect!["2"]);
}

// -- text_in_char_range with multi-byte --

#[test]
fn text_in_char_range_multibyte() {
    let b = buf("你好世界", 0, 0);
    check(&b.text_in_char_range(1, 3), expect!["好世"]);
}

// -- delete_char_range with multi-byte --

#[test]
fn delete_char_range_multibyte() {
    let mut b = buf("你好世界", 0, 0);
    b.delete_char_range(1, 3);
    check(&state(&b), expect![[r#"(0,1) "你界""#]]);
}
