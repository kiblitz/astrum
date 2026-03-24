use super::helpers::*;
use expect_test::expect;

#[test]
fn find_file_with_entries() {
    let ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
        ("tests", true, 0),
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /home/user/project\                         │
          ~             │./                                            │
          ~             │src/                                          │
          ~             │tests/                                        │
          ~             │Cargo.toml                                    │
          ~             │README.md                                     │
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
fn find_file_with_query() {
    let mut ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
    ]);
    ff.input = "Car".to_string();
    ff.filtered = vec![2]; // Only Cargo.toml matches.
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /home/user/project\Car                      │
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
          ~             │                                              │
          ~             │                                              │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_selected_second() {
    let mut ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
        ("tests", true, 0),
    ]);
    ff.selected = 1;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /home/user/project\                         │
          ~             │./                                            │
          ~             │src/                                          │
          ~             │tests/                                        │
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
fn find_file_no_matches() {
    let mut ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
    ]);
    ff.input = "zzz".to_string();
    ff.filtered = vec![]; // No matches.
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /home/user/project\zzz                      │
          ~             │  Press Enter to create "zzz"                 │
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
          ~             │                                              │
          ~             │                                              │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_long_path() {
    let ff = make_find_file("/home/user/very/deeply/nested/project/directory/structure", vec![
        (".", true, 0),
        ("file.rs", false, 100),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> y/deeply/nested/project/directory/structure\│
          ~             │./                                            │
          ~             │file.rs                                       │
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
          ~             │                                              │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_small_terminal() {
    let ff = make_find_file("/test", vec![
        (".", true, 0),
        ("a.rs", false, 10),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(40, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          ┌ Find File ───────────────────────┐
          │> /test\                          │
          │./                                │
          │a.rs                              │
          │                                  │
          │                                  │
          │                                  │
          │                                  │
          │                                  │
         N└──────────────────────────────────┘1
        SPC for leader | : for commands | SPC q"#]]);
}
