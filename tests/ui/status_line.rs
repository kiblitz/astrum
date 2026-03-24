use super::helpers::*;
use astrum::buffer::Buffer;
use astrum::input::Mode;
use expect_test::expect;

#[test]
fn status_normal_mode() {
    let buf = make_buffer("hello world\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
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
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_insert_mode() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
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
         INSERT  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_visual_mode() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_visual_anchor(0, 0);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
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
         VISUAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_command_mode() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("w");
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
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
         COMMAND  main.rs                                                           1:1
        :w"#]]);
}

#[test]
fn status_search_mode() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("hello", astrum::action::SearchDirection::Forward);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
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
         SEARCH  main.rs                                                            1:1
        /hello"#]]);
}

#[test]
fn status_modified_file() {
    let mut buf = make_buffer("hello\n", "main.rs");
    buf.modified = true;
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs [+]
          1  hello
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
         NORMAL  main.rs [+]                                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_scratch_buffer() {
    let buf = Buffer::new_scratch();
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          [scratch]
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
         NORMAL  [scratch]                                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_cursor_position_middle() {
    let buf = make_buffer_at("aaa\nbbb\nccc\nddd\n", "test.rs", 2, 1);
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  aaa
          2  bbb
          3  ccc
          4  ddd
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
fn status_welcome_screen() {
    // No buffers — welcome screen status.
    let state = RenderState::default();
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum



                                 _        _
                                / \   ___| |_ _ __ _   _ _ __ ___
                               / _ \ / __| __| '__| | | | '_ ` _ \
                              / ___ \\__ \ |_| |  | |_| | | | | | |
                             /_/   \_\___/\__|_|   \__,_|_| |_| |_|

                                             v0.1.0


                                       SPC f f  Browse files
                                           :q       Quit

                                 Press SPC for leader key commands





         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_narrow_terminal() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         NORMAL  main.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn status_insert_mode_modified() {
    let mut buf = make_buffer("hello\n", "main.rs");
    buf.modified = true;
    let state = RenderState::default()
        .with_buffer(buf)
        .with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs [+]
          1  hello
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
         INSERT  main.rs [+]                                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_visual_mode_modified() {
    let mut buf = make_buffer("hello\n", "main.rs");
    buf.modified = true;
    let state = RenderState::default()
        .with_buffer(buf)
        .with_visual_anchor(0, 0);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs [+]
          1  hello
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
         VISUAL  main.rs [+]                                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_cursor_far_col() {
    let long_line = "a".repeat(200) + "\n";
    let buf = make_buffer_at(&long_line, "test.rs", 0, 150);
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          test.rs
          1  aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
          ~
          ~
          ~
          ~
         NORMAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_large_file() {
    // 100 lines — cursor at line 50
    let text = (1..=100).map(|i| format!("line {}\n", i)).collect::<String>();
    let buf = make_buffer_at(&text, "big.rs", 49, 3);
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          big.rs
          1  line 1
          2  line 2
          3  line 3
          4  line 4
          5  line 5
         NORMAL  big.rs                                                             1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_very_long_filename() {
    let buf = make_buffer("x\n", "this_is_a_really_long_filename_that_might_cause_status_line_overflow.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          this_is_a_really_long_filename_that_might_cause_status_line_overflow.rs
          1  x
          ~
          ~
          ~
          ~
         NORMAL  this_is_a_really_long_filename_that_might_cause_status_line_overflow.rs
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn status_command_mode_modified() {
    let mut buf = make_buffer("hello\n", "main.rs");
    buf.modified = true;
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("wq");
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs [+]
          1  hello
          ~
          ~
          ~
          ~
         COMMAND  main.rs [+]                                                       1:1
        :wq"#]]);
}

#[test]
fn status_very_narrow_20_cols() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(20, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         NORMAL  main.rs  1:
        SPC for leader | : f"#]]);
}

#[test]
fn status_search_backward_mode() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("test", astrum::action::SearchDirection::Backward);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         SEARCH  main.rs                                                            1:1
        ?test"#]]);
}
