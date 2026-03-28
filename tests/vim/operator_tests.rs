//! Tests for operator+motion composition and visual mode operations.
//!
//! These simulate the editor's operator logic at the buffer level:
//! save cursor → apply motion → compute range → operate on range.

use astrum::action::Action;
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

/// Apply a motion to the buffer (mirrors editor's apply_motion).
fn apply_motion(b: &mut TextBuffer, action: &Action) {
    match action {
        Action::MoveUp => b.move_up(),
        Action::MoveDown => b.move_down(),
        Action::MoveLeft => b.move_left(),
        Action::MoveRight => b.move_right(),
        Action::MoveWordForward => b.move_word_forward(),
        Action::MoveWordEnd => b.move_word_end(),
        Action::MoveWordBackward => b.move_word_backward(),
        Action::MoveBigWordForward => b.move_big_word_forward(),
        Action::MoveBigWordEnd => b.move_big_word_end(),
        Action::MoveBigWordBackward => b.move_big_word_backward(),
        Action::MoveToLineStart => b.move_to_line_start(),
        Action::MoveToLineEnd => b.move_to_line_end(),
        Action::MoveToFirstLine => b.move_to_first_line(),
        Action::MoveToLastLine => b.move_to_last_line(),
        _ => {}
    }
}

/// Compute the operator range for a motion applied `count` times.
/// Returns (start_char_idx, end_char_idx) suitable for delete/yank/change.
fn operator_range(b: &mut TextBuffer, motion: &Action, count: usize) -> (usize, usize) {
    let start_line = b.cursor.line;
    let start_col = b.cursor.col;

    for _ in 0..count {
        let prev = (b.cursor.line, b.cursor.col);
        apply_motion(b, motion);
        if (b.cursor.line, b.cursor.col) == prev {
            break;
        }
    }

    let end_line = b.cursor.line;
    let end_col = b.cursor.col;

    if motion.is_linewise_motion() {
        let first = start_line.min(end_line);
        let last = start_line.max(end_line);
        let range = b.linewise_range(first, last);
        b.cursor.line = first;
        b.cursor.col = 0;
        range
    } else {
        let start = b.char_idx_at(start_line, start_col);
        let end = b.char_idx_at(end_line, end_col);
        let (min, max) = if start <= end { (start, end) } else { (end, start) };
        let mut to = if motion.is_exclusive_motion() { max } else { max + 1 };

        if min == to && motion.is_exclusive_motion() && !motion.is_backward_motion()
            && b.current_line_len() > 0
        {
            to = (min + 1).min(b.rope.len_chars());
        }

        let to = to.min(b.rope.len_chars());

        if start <= end {
            b.cursor.line = start_line;
            b.cursor.col = start_col;
        } else {
            b.cursor.line = end_line;
            b.cursor.col = end_col;
        }

        (min, to)
    }
}

/// Simulate delete operator + motion (like d{motion}).
fn delete_with_motion(b: &mut TextBuffer, motion: &Action, count: usize) -> String {
    let orig_line = b.cursor.line;
    let orig_col = b.cursor.col;
    let (start, end) = operator_range(b, motion, count);
    if start >= end {
        return String::new();
    }
    let yanked = b.text_in_char_range(start, end);
    b.cursor.line = orig_line;
    b.cursor.col = orig_col;
    b.delete_char_range(start, end);
    yanked
}

/// Simulate yank operator + motion (like y{motion}).
fn yank_with_motion(b: &mut TextBuffer, motion: &Action, count: usize) -> String {
    let orig_line = b.cursor.line;
    let orig_col = b.cursor.col;
    let (start, end) = operator_range(b, motion, count);
    // Yank doesn't modify buffer, restore cursor.
    b.cursor.line = orig_line;
    b.cursor.col = orig_col;
    b.text_in_char_range(start, end)
}

/// Simulate change operator + motion (like c{motion}).
fn change_with_motion(b: &mut TextBuffer, motion: &Action, count: usize) -> String {
    let orig_line = b.cursor.line;
    let orig_col = b.cursor.col;
    let (start, end) = operator_range(b, motion, count);
    if start >= end {
        return String::new();
    }
    let yanked = b.text_in_char_range(start, end);
    b.cursor.line = orig_line;
    b.cursor.col = orig_col;
    b.delete_char_range(start, end);
    yanked
}

/// Simulate visual mode: select from anchor to cursor, then operate.
fn visual_range(b: &TextBuffer, anchor_line: usize, anchor_col: usize) -> (usize, usize) {
    let anchor = b.char_idx_at(anchor_line, anchor_col);
    let cursor = b.char_idx_at(b.cursor.line, b.cursor.col);
    let start = anchor.min(cursor);
    let end = (anchor.max(cursor) + 1).min(b.rope.len_chars());
    (start, end)
}

// ======================================================================
// Operator + motion: delete (d)
// ======================================================================

#[test]
fn dw_delete_word() {
    let mut b = buf("hello world", 0, 0);
    delete_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn d2w_delete_two_words() {
    let mut b = buf("one two three", 0, 0);
    delete_with_motion(&mut b, &Action::MoveWordForward, 2);
    check(&state(&b), expect![[r#"(0,0) "three""#]]);
}

#[test]
fn de_delete_to_word_end() {
    let mut b = buf("hello world", 0, 0);
    delete_with_motion(&mut b, &Action::MoveWordEnd, 1);
    check(&state(&b), expect![[r#"(0,0) " world""#]]);
}

#[test]
fn db_delete_word_backward() {
    let mut b = buf("hello world", 0, 6);
    delete_with_motion(&mut b, &Action::MoveWordBackward, 1);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn d_dollar_delete_to_line_end() {
    let mut b = buf("hello world", 0, 5);
    delete_with_motion(&mut b, &Action::MoveToLineEnd, 1);
    check(&state(&b), expect![[r#"(0,4) "hello""#]]);
}

#[test]
fn d0_delete_to_line_start() {
    let mut b = buf("hello world", 0, 6);
    delete_with_motion(&mut b, &Action::MoveToLineStart, 1);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn dj_delete_two_lines() {
    let mut b = buf("aaa\nbbb\nccc", 0, 0);
    delete_with_motion(&mut b, &Action::MoveDown, 1);
    check(&state(&b), expect![[r#"(0,0) "ccc""#]]);
}

#[test]
fn dk_delete_two_lines_upward() {
    let mut b = buf("aaa\nbbb\nccc", 2, 0);
    delete_with_motion(&mut b, &Action::MoveUp, 1);
    // dk from line 2: deletes lines 1-2 (linewise), leaves "aaa\n"
    check(&state(&b), expect![[r#"(0,2) "aaa\n""#]]);
}

#[test]
fn d2j_delete_three_lines() {
    let mut b = buf("aaa\nbbb\nccc\nddd", 0, 0);
    delete_with_motion(&mut b, &Action::MoveDown, 2);
    check(&state(&b), expect![[r#"(0,0) "ddd""#]]);
}

#[test]
fn dgg_delete_to_top() {
    let mut b = buf("aaa\nbbb\nccc", 2, 0);
    delete_with_motion(&mut b, &Action::MoveToFirstLine, 1);
    check(&state(&b), expect![[r#"(0,0) """#]]);
}

#[test]
#[allow(non_snake_case)]
fn dG_delete_to_bottom() {
    let mut b = buf("aaa\nbbb\nccc", 0, 0);
    delete_with_motion(&mut b, &Action::MoveToLastLine, 1);
    check(&state(&b), expect![[r#"(0,0) """#]]);
}

#[test]
fn dl_at_end_of_line() {
    // dl at end of line should delete the last char
    let mut b = buf("abc", 0, 2);
    delete_with_motion(&mut b, &Action::MoveRight, 1);
    check(&state(&b), expect![[r#"(0,1) "ab""#]]);
}

#[test]
fn dl_on_empty_line() {
    let mut b = buf("\nabc", 0, 0);
    let yanked = delete_with_motion(&mut b, &Action::MoveRight, 1);
    // Empty line has no chars — nothing to delete
    assert!(yanked.is_empty());
    check(&state(&b), expect![[r#"(0,0) "\nabc""#]]);
}

#[test]
fn dh_at_start_of_line() {
    let mut b = buf("abc", 0, 0);
    let yanked = delete_with_motion(&mut b, &Action::MoveLeft, 1);
    assert!(yanked.is_empty());
    check(&state(&b), expect![[r#"(0,0) "abc""#]]);
}

#[test]
fn dw_across_lines() {
    let mut b = buf("hello\nworld", 0, 3);
    delete_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&state(&b), expect![[r#"(0,3) "helworld""#]]);
}

// ======================================================================
// Operator + motion: yank (y)
// ======================================================================

#[test]
fn yw_yank_word() {
    let mut b = buf("hello world", 0, 0);
    let yanked = yank_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&yanked, expect!["hello "]);
    // Buffer unchanged, cursor unchanged
    check(&state(&b), expect![[r#"(0,0) "hello world""#]]);
}

#[test]
fn yj_yank_two_lines() {
    let mut b = buf("aaa\nbbb\nccc", 0, 0);
    let yanked = yank_with_motion(&mut b, &Action::MoveDown, 1);
    check(&yanked, expect!["aaa\nbbb\n"]);
    check(&state(&b), expect![[r#"(0,0) "aaa\nbbb\nccc""#]]);
}

#[test]
fn y2j_yank_three_lines() {
    let mut b = buf("aaa\nbbb\nccc\nddd", 0, 0);
    let yanked = yank_with_motion(&mut b, &Action::MoveDown, 2);
    check(&yanked, expect!["aaa\nbbb\nccc\n"]);
    check(&state(&b), expect![[r#"(0,0) "aaa\nbbb\nccc\nddd""#]]);
}

#[test]
fn ye_yank_to_word_end() {
    let mut b = buf("hello world", 0, 0);
    let yanked = yank_with_motion(&mut b, &Action::MoveWordEnd, 1);
    check(&yanked, expect!["hello"]);
}

#[test]
fn y_dollar_yank_to_line_end() {
    let mut b = buf("hello world", 0, 5);
    let yanked = yank_with_motion(&mut b, &Action::MoveToLineEnd, 1);
    // $ is inclusive, so from col 5 (space) through col 10 (d)
    check(&yanked, expect![" world"]);
}

// ======================================================================
// Operator + motion: change (c)
// ======================================================================

#[test]
fn cw_change_word() {
    let mut b = buf("hello world", 0, 0);
    let yanked = change_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&yanked, expect!["hello "]);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
}

#[test]
fn ce_change_to_word_end() {
    let mut b = buf("hello world", 0, 0);
    let yanked = change_with_motion(&mut b, &Action::MoveWordEnd, 1);
    check(&yanked, expect!["hello"]);
    check(&state(&b), expect![[r#"(0,0) " world""#]]);
}

#[test]
fn c_dollar_change_to_line_end() {
    let mut b = buf("hello world", 0, 5);
    let yanked = change_with_motion(&mut b, &Action::MoveToLineEnd, 1);
    // $ is inclusive, from col 5 (space) through end
    check(&yanked, expect![" world"]);
    check(&state(&b), expect![[r#"(0,4) "hello""#]]);
}

// ======================================================================
// Linewise operators: dd, cc, yy
// ======================================================================

#[test]
fn dd_single_line() {
    let mut b = buf("aaa\nbbb\nccc", 1, 0);
    let (start, end) = b.linewise_range(1, 1);
    let yanked = b.text_in_char_range(start, end);
    b.delete_char_range(start, end);
    check(&yanked, expect!["bbb\n"]);
    check(&state(&b), expect![[r#"(1,0) "aaa\nccc""#]]);
}

#[test]
fn dd_2_lines() {
    let mut b = buf("aaa\nbbb\nccc\nddd", 0, 0);
    let last = (0 + 2 - 1).min(b.line_count().saturating_sub(1));
    let (start, end) = b.linewise_range(0, last);
    let yanked = b.text_in_char_range(start, end);
    b.delete_char_range(start, end);
    check(&yanked, expect!["aaa\nbbb\n"]);
    check(&state(&b), expect![[r#"(0,0) "ccc\nddd""#]]);
}

#[test]
fn yy_single_line() {
    let b = buf("aaa\nbbb\nccc", 1, 2);
    let (start, end) = b.linewise_range(1, 1);
    let yanked = b.text_in_char_range(start, end);
    check(&yanked, expect!["bbb\n"]);
    // Buffer and cursor unchanged
    check(&state(&b), expect![[r#"(1,2) "aaa\nbbb\nccc""#]]);
}

#[test]
fn cc_single_line_undo() {
    let mut b = buf("hello\nworld", 0, 3);
    // Simulate cc: single save_undo, delete line range, insert blank line
    b.save_undo();
    let (start, end) = b.linewise_range(0, 0);
    b.rope.remove(start..end);
    let pos = start.min(b.rope.len_chars());
    b.rope.insert_char(pos, '\n');
    b.cursor.line = b.rope.char_to_line(pos);
    b.cursor.col = 0;
    check(&state(&b), expect![[r#"(0,0) "\nworld""#]]);
    // Single undo restores
    b.undo();
    check(&state(&b), expect![[r#"(0,3) "hello\nworld""#]]);
    // No second undo entry (cursor should stay the same)
    b.undo();
    check(&state(&b), expect![[r#"(0,3) "hello\nworld""#]]);
}

// ======================================================================
// Visual mode operations
// ======================================================================

#[test]
fn visual_delete_forward() {
    let mut b = buf("hello world", 0, 2);
    // Visual select from col 2 to col 7 (anchor=2, cursor=7 → "llo wo")
    b.cursor.col = 7;
    let (start, end) = visual_range(&b, 0, 2);
    let yanked = b.text_in_char_range(start, end);
    b.delete_char_range(start, end);
    check(&yanked, expect!["llo wo"]);
    check(&state(&b), expect![[r#"(0,2) "herld""#]]);
}

#[test]
fn visual_delete_backward() {
    let mut b = buf("hello world", 0, 7);
    // Visual select from col 7 to col 2 (anchor=7, cursor=2 → "llo wo")
    b.cursor.col = 2;
    let (start, end) = visual_range(&b, 0, 7);
    let yanked = b.text_in_char_range(start, end);
    b.delete_char_range(start, end);
    check(&yanked, expect!["llo wo"]);
    check(&state(&b), expect![[r#"(0,2) "herld""#]]);
}

#[test]
fn visual_yank() {
    let mut b = buf("hello world", 0, 0);
    b.cursor.col = 4;
    let (start, end) = visual_range(&b, 0, 0);
    let yanked = b.text_in_char_range(start, end);
    check(&yanked, expect!["hello"]);
    // Buffer unchanged
    check(&state(&b), expect![[r#"(0,4) "hello world""#]]);
}

#[test]
fn visual_change() {
    let mut b = buf("hello world", 0, 0);
    b.cursor.col = 4;
    let (start, end) = visual_range(&b, 0, 0);
    let yanked = b.text_in_char_range(start, end);
    b.delete_char_range(start, end);
    check(&yanked, expect!["hello"]);
    check(&state(&b), expect![[r#"(0,0) " world""#]]);
}

#[test]
fn visual_multiline() {
    let mut b = buf("aaa\nbbb\nccc", 0, 1);
    // Select from (0,1) to (1,1): "aa\nbb"
    b.cursor.line = 1;
    b.cursor.col = 1;
    let (start, end) = visual_range(&b, 0, 1);
    let yanked = b.text_in_char_range(start, end);
    check(&yanked, expect!["aa\nbb"]);
    b.delete_char_range(start, end);
    check(&state(&b), expect![[r#"(0,1) "ab\nccc""#]]);
}

#[test]
fn visual_single_char() {
    let mut b = buf("hello", 0, 2);
    // Anchor = cursor = same position → select one char
    let (start, end) = visual_range(&b, 0, 2);
    let yanked = b.text_in_char_range(start, end);
    check(&yanked, expect!["l"]);
    b.delete_char_range(start, end);
    check(&state(&b), expect![[r#"(0,2) "helo""#]]);
}

#[test]
fn visual_whole_line() {
    let mut b = buf("hello\nworld", 0, 0);
    b.cursor.col = 4;
    let (start, end) = visual_range(&b, 0, 0);
    let yanked = b.text_in_char_range(start, end);
    check(&yanked, expect!["hello"]);
}

#[test]
fn visual_delete_undo() {
    let mut b = buf("hello world", 0, 0);
    b.cursor.col = 4;
    let (start, end) = visual_range(&b, 0, 0);
    b.delete_char_range(start, end);
    check(&state(&b), expect![[r#"(0,0) " world""#]]);
    b.undo();
    // Undo restores to cursor position when save_undo was called (col 4)
    check(&state(&b), expect![[r#"(0,4) "hello world""#]]);
}

// ======================================================================
// Operator with counted motions
// ======================================================================

#[test]
fn d3l_delete_three_chars() {
    let mut b = buf("abcdefgh", 0, 2);
    delete_with_motion(&mut b, &Action::MoveRight, 3);
    check(&state(&b), expect![[r#"(0,2) "abfgh""#]]);
}

#[test]
fn d2w_delete_two_words_mid_line() {
    let mut b = buf("one two three four", 0, 4);
    delete_with_motion(&mut b, &Action::MoveWordForward, 2);
    check(&state(&b), expect![[r#"(0,4) "one four""#]]);
}

#[test]
fn y3w_yank_three_words() {
    let mut b = buf("one two three four", 0, 0);
    let yanked = yank_with_motion(&mut b, &Action::MoveWordForward, 3);
    check(&yanked, expect!["one two three "]);
}

// ======================================================================
// Edge cases
// ======================================================================

#[test]
fn dw_at_last_word_of_line() {
    let mut b = buf("hello world\nnext", 0, 6);
    delete_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&state(&b), expect![[r#"(0,6) "hello next""#]]);
}

#[test]
fn delete_undo_restores_cursor() {
    let mut b = buf("hello world", 0, 0);
    delete_with_motion(&mut b, &Action::MoveWordForward, 1);
    check(&state(&b), expect![[r#"(0,0) "world""#]]);
    b.undo();
    check(&state(&b), expect![[r#"(0,0) "hello world""#]]);
}

#[test]
#[allow(non_snake_case)]
fn dW_delete_big_word() {
    let mut b = buf("foo.bar baz", 0, 0);
    delete_with_motion(&mut b, &Action::MoveBigWordForward, 1);
    check(&state(&b), expect![[r#"(0,0) "baz""#]]);
}

#[test]
#[allow(non_snake_case)]
fn dE_delete_to_big_word_end() {
    let mut b = buf("foo.bar baz", 0, 0);
    delete_with_motion(&mut b, &Action::MoveBigWordEnd, 1);
    check(&state(&b), expect![[r#"(0,0) " baz""#]]);
}

#[test]
#[allow(non_snake_case)]
fn dB_delete_big_word_backward() {
    let mut b = buf("foo.bar baz", 0, 8);
    delete_with_motion(&mut b, &Action::MoveBigWordBackward, 1);
    check(&state(&b), expect![[r#"(0,0) "baz""#]]);
}
