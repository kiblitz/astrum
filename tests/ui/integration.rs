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

#[test]
fn full_frame_palette_overlay() {
    use astrum::action::Action;
    let palette = super::helpers::make_palette(vec![
        ("Save buffer", "SPC f s", Action::SaveBuffer),
        ("Open file browser", "SPC f f", Action::OpenFileBrowser),
        ("Split vertical", "SPC w v", Action::SplitVertical),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    let buf = make_buffer("fn main() {\n    println!(\"hello\");\n}\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  fn main() {
          2      println!("hello");
          3  }
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Save buffer                            SPC f s│
          ~             │Open file browser                      SPC f f│
          ~             │Split vertical                         SPC w v│
          ~             │Quit                                   SPC q q│
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

#[test]
fn full_frame_recent_picker_overlay() {
    let rp = super::helpers::make_recent_picker(vec![
        ("src/main.rs", false),
        ("src/lib.rs", false),
        ("Cargo.toml", false),
    ]);
    let buf = make_buffer("fn main() {}\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  fn main() {}
          ~
          ~
          ~             ┌ Recent Files ────────────────────────────────┐
          ~             │>                                             │
          ~             │src/main.rs                                   │
          ~             │src/lib.rs                                    │
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

#[test]
fn full_frame_recording_macro() {
    let buf = make_buffer("hello world\nfoo bar\n", "test.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recording_macro('a');
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          2  foo bar
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
        recording @a SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_status_message() {
    let buf = make_buffer("hello\n", "test.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_status("\"test.rs\" 1L, 6B written");
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
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
         NORMAL  test.rs                                                            1:1
        "test.rs" 1L, 6B written"#]]);
}

#[test]
fn full_frame_horizontal_split() {
    let buf1 = make_buffer("top content\n", "top.rs");
    let buf2 = make_buffer("bottom content\n", "bot.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.buffer_id = Some(buf2_id);
    }
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          top.rs  │  bot.rs
          1  top content
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         top.rs
        ────────────────────────────────────────────────────────────────────────────────
          1  bottom content
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         bot.rs
         NORMAL  top.rs                                                             1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_insert_mode() {
    let buf = make_buffer("hello world\n", "test.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_mode(astrum::input::Mode::Insert);
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
         INSERT  test.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_search_mode_active() {
    let buf = make_buffer("hello world\nfoo bar\n", "test.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("world", astrum::action::SearchDirection::Forward);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
          1  hello world
          2  foo bar
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
         SEARCH  test.rs                                                            1:1
        /world"#]]);
}

#[test]
fn full_frame_which_key_hints() {
    let buf = make_buffer("hello\n", "test.rs");
    let hints = vec![
        ("f".to_string(), "file".to_string()),
        ("b".to_string(), "buffer".to_string()),
        ("w".to_string(), "window".to_string()),
        ("q".to_string(), "quit".to_string()),
    ];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_pending_keys("SPC", hints);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.rs
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
         NORMAL  test.rs                                                            1:1
        SPC-  f:file b:buffer w:window q:quit"#]]);
}

#[test]
fn full_frame_modified_file() {
    let mut buf = make_buffer("unsaved changes\n", "draft.rs");
    buf.modified = true;
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          draft.rs [+]
          1  unsaved changes
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
         NORMAL  draft.rs [+]                                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_many_lines_scrolled() {
    let text = (1..=50).map(|i| format!("line {:>3}\n", i)).collect::<String>();
    let mut buf = make_buffer_at(&text, "long.rs", 30, 0);
    buf.scroll_offset = 20;
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          long.rs
          1  line   1
          2  line   2
          3  line   3
          4  line   4
          5  line   5
          6  line   6
          7  line   7
          8  line   8
          9  line   9
         10  line  10
         11  line  11
         12  line  12
         13  line  13
         14  line  14
         15  line  15
         16  line  16
         17  line  17
         18  line  18
         19  line  19
         20  line  20
         21  line  21
         NORMAL  long.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_config_error_welcome() {
    let state = RenderState::default()
        .with_config_error("Config error: parse error at line 5");
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

                                Config error: parse error at line 5
                                    Using default configuration


         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_narrow_40x12() {
    let buf = make_buffer("fn main() {\n    println!(\"hello\");\n}\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(40, 12, &state);
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
         NORMAL  main.rs                    1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn full_frame_wide_160x24() {
    let buf = make_buffer("fn main() {}\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(160, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  fn main() {}
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
         NORMAL  main.rs                                                                                                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn full_frame_split_with_search() {
    let buf1 = make_buffer("foo bar foo\n", "left.rs");
    let buf2 = make_buffer("baz foo qux\n", "right.rs");
    let buf1_id = buf1.id;
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_search_matches(buf1_id, vec![(0, 0, 3), (0, 8, 11)], Some(0))
        .with_search_matches(buf2_id, vec![(0, 4, 7)], None);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.buffer_id = Some(buf2_id);
    }
    let actual = render_to_string(80, 16, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  foo bar foo                        │  1  baz foo qux
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
fn full_frame_browser_in_split() {
    let buf = make_buffer("hello world\n", "main.rs");
    let mut fb = astrum::file_browser::FileBrowser::new(PathBuf::from("/project"));
    fb.set_entries(vec![
        make_dir_entry("src", true, 0),
        make_dir_entry("Cargo.toml", false, 2048),
    ]);
    let mut state = RenderState::default().with_buffer(buf);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    let state = state.with_file_browser(p2, fb);
    let actual = render_to_string(80, 16, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello world                        │  /project
          ~                                     │───────────────────────────────────────
          ~                                     │ >  src/                          dir
          ~                                     │    Cargo.toml                   2.0K
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
         main.rs                                │ main.rs
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}
