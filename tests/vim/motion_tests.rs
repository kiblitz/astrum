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

fn pos(b: &TextBuffer) -> String {
    format!("({},{})", b.cursor.line, b.cursor.col)
}

fn check(actual: &str, expect: Expect) {
    expect.assert_eq(actual);
}

// -- basic movement --

#[test]
fn move_left() {
    let mut b = buf("hello", 0, 3);
    b.move_left();
    check(&pos(&b), expect!["(0,2)"]);
}

#[test]
fn move_left_at_start() {
    let mut b = buf("hello", 0, 0);
    b.move_left();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn move_right() {
    let mut b = buf("hello", 0, 2);
    b.move_right();
    check(&pos(&b), expect!["(0,3)"]);
}

#[test]
fn move_right_clamps_at_last_char() {
    let mut b = buf("abc", 0, 2);
    b.move_right();
    check(&pos(&b), expect!["(0,2)"]);
}

#[test]
fn move_right_empty_line() {
    let mut b = buf("\nhello", 0, 0);
    b.move_right();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn move_up() {
    let mut b = buf("aaa\nbbb\nccc", 2, 1);
    b.move_up();
    check(&pos(&b), expect!["(1,1)"]);
}

#[test]
fn move_up_at_top() {
    let mut b = buf("hello", 0, 2);
    b.move_up();
    check(&pos(&b), expect!["(0,2)"]);
}

#[test]
fn move_down() {
    let mut b = buf("aaa\nbbb\nccc", 0, 1);
    b.move_down();
    check(&pos(&b), expect!["(1,1)"]);
}

#[test]
fn move_down_at_bottom() {
    let mut b = buf("hello", 0, 2);
    b.move_down();
    check(&pos(&b), expect!["(0,2)"]);
}

#[test]
fn move_down_clamps_col_to_shorter_line() {
    let mut b = buf("hello\nhi", 0, 4);
    b.move_down();
    check(&pos(&b), expect!["(1,1)"]);
}

#[test]
fn move_up_clamps_col_to_shorter_line() {
    let mut b = buf("hi\nhello", 1, 4);
    b.move_up();
    check(&pos(&b), expect!["(0,1)"]);
}

// -- line start/end --

#[test]
fn move_to_line_start() {
    let mut b = buf("hello", 0, 3);
    b.move_to_line_start();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn move_to_line_end() {
    let mut b = buf("hello", 0, 0);
    b.move_to_line_end();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn move_to_line_end_with_newline() {
    let mut b = buf("hello\nworld", 0, 0);
    b.move_to_line_end();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn move_to_line_end_empty_line() {
    let mut b = buf("hello\n\nworld", 1, 0);
    b.move_to_line_end();
    check(&pos(&b), expect!["(1,0)"]);
}

// -- first/last line --

#[test]
fn move_to_first_line() {
    let mut b = buf("aaa\nbbb\nccc", 2, 1);
    b.move_to_first_line();
    check(&pos(&b), expect!["(0,1)"]);
}

#[test]
fn move_to_last_line() {
    let mut b = buf("aaa\nbbb\nccc", 0, 1);
    b.move_to_last_line();
    check(&pos(&b), expect!["(2,1)"]);
}

#[test]
fn move_to_last_line_trailing_newline() {
    let mut b = buf("aaa\nbbb\n", 0, 0);
    b.move_to_last_line();
    check(&pos(&b), expect!["(1,0)"]);
}

// -- goto_line --

#[test]
fn goto_line_middle() {
    let mut b = buf("aaa\nbbb\nccc\nddd", 0, 0);
    b.goto_line(2);
    check(&pos(&b), expect!["(2,0)"]);
}

#[test]
fn goto_line_past_end() {
    let mut b = buf("aaa\nbbb", 0, 0);
    b.goto_line(99);
    check(&pos(&b), expect!["(1,0)"]);
}

// -- word forward --

#[test]
fn word_forward_simple() {
    let mut b = buf("hello world", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,6)"]);
}

#[test]
fn word_forward_punctuation_boundary() {
    let mut b = buf("main()", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn word_forward_space_then_punctuation() {
    let mut b = buf("main ()", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,5)"]);
}

#[test]
fn word_forward_from_punctuation() {
    let mut b = buf("foo.bar", 0, 3);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn word_forward_multiple_punctuation() {
    let mut b = buf("a::b", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,1)"]);
}

#[test]
fn word_forward_across_line() {
    let mut b = buf("end\nstart", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(1,0)"]);
}

#[test]
fn word_forward_at_end_of_file() {
    let mut b = buf("hello", 0, 3);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn word_forward_trailing_spaces() {
    let mut b = buf("hello   ", 0, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(0,7)"]);
}

#[test]
fn word_forward_last_line_stays() {
    let mut b = buf("aaa\nbbb", 1, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(1,2)"]);
}

#[test]
fn word_forward_last_line_multi_word() {
    let mut b = buf("aaa\nhello world", 1, 0);
    b.move_word_forward();
    check(&pos(&b), expect!["(1,6)"]);
}

// -- word backward --

#[test]
fn word_backward_simple() {
    let mut b = buf("hello world", 0, 6);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn word_backward_punctuation() {
    let mut b = buf("foo.bar", 0, 4);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,3)"]);
}

#[test]
fn word_backward_from_word_start() {
    let mut b = buf("hello world", 0, 6);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn word_backward_skips_punctuation_class() {
    let mut b = buf("a::b", 0, 3);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,1)"]);
}

#[test]
fn word_backward_across_line() {
    let mut b = buf("hello\nworld", 1, 0);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn word_backward_at_start_of_file() {
    let mut b = buf("hello", 0, 0);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn word_backward_mid_word() {
    let mut b = buf("hello world", 0, 8);
    b.move_word_backward();
    check(&pos(&b), expect!["(0,6)"]);
}

// -- word end --

#[test]
fn word_end_simple() {
    let mut b = buf("hello world", 0, 0);
    b.move_word_end();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn word_end_at_end_of_word() {
    let mut b = buf("hello world", 0, 4);
    b.move_word_end();
    check(&pos(&b), expect!["(0,10)"]);
}

#[test]
fn word_end_punctuation() {
    let mut b = buf("foo..bar", 0, 0);
    b.move_word_end();
    check(&pos(&b), expect!["(0,2)"]);
}

#[test]
fn word_end_from_punctuation() {
    let mut b = buf("foo..bar", 0, 3);
    b.move_word_end();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn word_end_across_line() {
    let mut b = buf("hi\nworld", 0, 1);
    b.move_word_end();
    check(&pos(&b), expect!["(1,4)"]);
}

#[test]
fn word_end_at_file_end() {
    let mut b = buf("hi", 0, 1);
    b.move_word_end();
    check(&pos(&b), expect!["(0,1)"]);
}

// -- big word motions --

#[test]
fn big_word_forward() {
    let mut b = buf("foo.bar baz", 0, 0);
    b.move_big_word_forward();
    check(&pos(&b), expect!["(0,8)"]);
}

#[test]
fn big_word_forward_across_line() {
    let mut b = buf("foo.bar\nbaz", 0, 0);
    b.move_big_word_forward();
    check(&pos(&b), expect!["(1,0)"]);
}

#[test]
fn big_word_backward() {
    let mut b = buf("foo.bar baz", 0, 8);
    b.move_big_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn big_word_backward_across_line() {
    let mut b = buf("foo.bar\nbaz", 1, 0);
    b.move_big_word_backward();
    check(&pos(&b), expect!["(0,0)"]);
}

#[test]
fn big_word_end() {
    let mut b = buf("foo.bar baz", 0, 0);
    b.move_big_word_end();
    check(&pos(&b), expect!["(0,6)"]);
}

#[test]
fn big_word_end_across_line() {
    let mut b = buf("hi\nfoo.bar", 0, 1);
    b.move_big_word_end();
    check(&pos(&b), expect!["(1,6)"]);
}

// -- page/half-page --

#[test]
fn page_down() {
    let mut b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj", 0, 0);
    b.page_down(5);
    check(&pos(&b), expect!["(5,0)"]);
}

#[test]
fn page_up() {
    let mut b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj", 7, 0);
    b.page_up(5);
    check(&pos(&b), expect!["(7,0)"]);
}

#[test]
fn page_down_clamps() {
    let mut b = buf("a\nb\nc", 0, 0);
    b.page_down(100);
    check(&pos(&b), expect!["(2,0)"]);
}

#[test]
fn page_up_clamps() {
    let mut b = buf("a\nb\nc", 1, 0);
    b.page_up(100);
    check(&pos(&b), expect!["(1,0)"]);
}

#[test]
fn half_page_down() {
    let mut b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj", 0, 0);
    b.half_page_down(10);
    check(&pos(&b), expect!["(5,0)"]);
}

#[test]
fn half_page_up() {
    let mut b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj", 8, 0);
    b.half_page_up(10);
    check(&pos(&b), expect!["(8,0)"]);
}

// -- cursor clamping --

#[test]
fn clamp_cursor_normal_mode() {
    let mut b = buf("hello", 0, 10);
    b.clamp_cursor();
    check(&pos(&b), expect!["(0,4)"]);
}

#[test]
fn clamp_cursor_empty_line() {
    let mut b = buf("hello\n\nworld", 1, 5);
    b.clamp_cursor();
    check(&pos(&b), expect!["(1,0)"]);
}

#[test]
fn clamp_cursor_past_last_line() {
    let mut b = buf("hello", 5, 0);
    b.clamp_cursor();
    check(&pos(&b), expect!["(0,0)"]);
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

#[test]
fn linewise_range_all_lines() {
    let b = buf("aaa\nbbb\nccc", 0, 0);
    let (start, end) = b.linewise_range(0, 2);
    check(&format!("{}..{}", start, end), expect!["0..11"]);
}

// -- char_idx_at --

#[test]
fn char_idx_at_basic() {
    let b = buf("hello\nworld", 0, 0);
    check(&b.char_idx_at(0, 3).to_string(), expect!["3"]);
    check(&b.char_idx_at(1, 2).to_string(), expect!["8"]);
}

#[test]
fn char_idx_at_clamped() {
    let b = buf("hi\nworld", 0, 0);
    // col past end of line should clamp
    check(&b.char_idx_at(0, 99).to_string(), expect!["2"]);
}

#[test]
fn char_idx_at_past_last_line() {
    let b = buf("hello", 0, 0);
    check(&b.char_idx_at(99, 0).to_string(), expect!["5"]);
}
