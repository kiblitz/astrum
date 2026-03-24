use super::helpers::*;
use expect_test::expect;

#[test]
fn editor_single_line() {
    let buf = make_buffer("hello world\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         NORMAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn editor_multiple_lines() {
    let buf = make_buffer("line one\nline two\nline three\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  line one
          2  line two
          3  line three
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         NORMAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn editor_empty_buffer() {
    let buf = make_buffer("", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         NORMAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn editor_tilde_past_eof() {
    let buf = make_buffer("one\ntwo\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 10, &state);
    check(&actual, expect![[r#"
          test.rs
          1  one
          2  two
          ~
          ~
          ~
          ~
          ~
         NORMAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn editor_line_number_width_small() {
    // Few lines — gutter should be minimum 3 chars wide.
    let buf = make_buffer("a\nb\nc\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 10, &state);
    check(&actual, expect![[r#"
          test.rs
          1  a
          2  b
          3  c
          ~
          ~
          ~
          ~
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_current_line_highlighted() {
    let buf = make_buffer_at("aaa\nbbb\nccc\n", "test.rs", 1, 0);
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 10, &state);
    check(&actual, expect![[r#"
          test.rs
          1  aaa
          2  bbb
          3  ccc
          ~
          ~
          ~
          ~
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_scroll_offset() {
    // Buffer with many lines, scrolled down.
    let text = (1..=20).map(|i| format!("line {}\n", i)).collect::<String>();
    let mut buf = make_buffer_at(&text, "test.rs", 15, 0);
    buf.scroll_offset = 10;
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 10, &state);
    check(&actual, expect![[r#"
          test.rs
          1  line 1
          2  line 2
          3  line 3
          4  line 4
          5  line 5
          6  line 6
          7  line 7
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_long_line_no_wrap() {
    let buf = make_buffer("this is a very long line that should extend past the viewport boundary without wrapping\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  this is a very long line that shoul
          ~
          ~
          ~
          ~
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_cursor_at_end() {
    let buf = make_buffer_at("hello\n", "test.rs", 0, 4);
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello
          ~
          ~
          ~
          ~
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_narrow_width() {
    let buf = make_buffer("hello world\n", "test.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(20, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          ~
          ~
          ~
          ~
         NORMAL  test.rs  1:
        SPC for leader | : f"#]]);
}

#[test]
fn editor_with_search_highlights() {
    let buf = make_buffer("foo bar foo baz foo\n", "test.rs");
    let buf_id = buf.id;
    let matches = vec![(0, 0, 3), (0, 8, 11), (0, 16, 19)];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search_matches(buf_id, matches, Some(1));
    let actual = render_to_string(40, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  foo bar foo baz foo
          ~
          ~
          ~
          ~
         NORMAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn editor_with_visual_selection() {
    let mut buf = make_buffer("hello world\nfoo bar\n", "test.rs");
    buf.cursor.line = 0;
    buf.cursor.col = 8;
    let state = RenderState::default()
        .with_buffer(buf)
        .with_visual_anchor(0, 2);
    let actual = render_to_string(40, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          2  foo bar
          ~
          ~
          ~
         VISUAL  test.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}
