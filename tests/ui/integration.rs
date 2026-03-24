use super::helpers::*;
use astrum::pane::SplitDirection;
use expect_test::expect;
use std::path::PathBuf;

#[test]
fn full_frame_welcome_80x24() {
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
fn full_frame_single_buffer() {
    let buf = make_buffer(
        "fn main() {\n    println!(\"hello\");\n}\n",
        "main.rs",
    );
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  fn main() {
          2      println!("hello");
          3  }
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
fn full_frame_two_panes_vertical() {
    let buf1 = make_buffer("let x = 1;\nlet y = 2;\n", "left.rs");
    let buf2 = make_buffer("fn foo() {}\nfn bar() {}\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.buffer_id = Some(buf2_id);
    }
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  let x = 1;                         │  1  fn foo() {}
          2  let y = 2;                         │  2  fn bar() {}
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
         left.rs                                │ right.rs
         NORMAL  left.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_command_mode() {
    let buf = make_buffer("hello world\n", "test.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("w");
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
         COMMAND  test.rs                                                           1:1
        :w"#]]);
}

#[test]
fn full_frame_search_with_matches() {
    let buf = make_buffer("foo bar foo\nbaz foo qux\n", "test.rs");
    let buf_id = buf.id;
    let matches = vec![(0, 0, 3), (0, 8, 11), (1, 4, 7)];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search_matches(buf_id, matches, Some(0));
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  foo bar foo
          2  baz foo qux
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
fn full_frame_visual_selection() {
    let mut buf = make_buffer("hello world\nfoo bar baz\nqux quux\n", "test.rs");
    buf.cursor.line = 1;
    buf.cursor.col = 6;
    let state = RenderState::default()
        .with_buffer(buf)
        .with_visual_anchor(0, 3);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          2  foo bar baz
          3  qux quux
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
         VISUAL  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_file_browser() {
    let mut fb = astrum::file_browser::FileBrowser::new(PathBuf::from("/home/user/project"));
    fb.set_entries(vec![
        make_dir_entry("src", true, 0),
        make_dir_entry("tests", true, 0),
        make_dir_entry("Cargo.toml", false, 2048),
        make_dir_entry("README.md", false, 512),
    ]);
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
          /home/user/project
        ────────────────────────────────────────────────────────────────────────────────
         >  src/                                                                   dir
            tests/                                                                 dir
            Cargo.toml                                                            2.0K
            README.md                                                             512B















         FILES  1/4       j/k:nav  enter:open  h/-:up  /:filter  n:new  ~:home  q:close
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_find_file_overlay() {
    let buf = make_buffer("fn main() {}\n", "main.rs");
    let ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  fn main() {}
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /home/user/project\                         │
          ~             │./                                            │
          ~             │src/                                          │
          ~             │Cargo.toml                                    │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             │                                              │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}
