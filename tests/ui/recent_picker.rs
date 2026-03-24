use super::helpers::*;
use expect_test::expect;

#[test]
fn recent_empty_list() {
    let rp = make_recent_picker(vec![]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Recent Files ────────────────────────────────┐
          ~             │>                                             │
          ~             │  No recent files                             │
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
fn recent_with_files() {
    let rp = make_recent_picker(vec![
        ("src/main.rs", false),
        ("src/lib.rs", false),
        ("Cargo.toml", false),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
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
fn recent_with_dirs() {
    let rp = make_recent_picker(vec![
        ("src", true),
        ("tests", true),
        ("README.md", false),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Recent Files ────────────────────────────────┐
          ~             │>                                             │
          ~             │src/                                          │
          ~             │tests/                                        │
          ~             │README.md                                     │
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
fn recent_selected_second() {
    let mut rp = make_recent_picker(vec![
        ("main.rs", false),
        ("lib.rs", false),
        ("utils.rs", false),
    ]);
    rp.selected = 1;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Recent Files ────────────────────────────────┐
          ~             │>                                             │
          ~             │main.rs                                       │
          ~             │lib.rs                                        │
          ~             │utils.rs                                      │
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
fn recent_with_query() {
    let mut rp = make_recent_picker(vec![
        ("main.rs", false),
        ("lib.rs", false),
        ("Cargo.toml", false),
    ]);
    rp.query = "lib".to_string();
    rp.filtered = vec![1]; // Only lib.rs matches.
    rp.selected = 0;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Recent Files ────────────────────────────────┐
          ~             │> lib                                         │
          ~             │lib.rs                                        │
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
fn recent_small_terminal() {
    let rp = make_recent_picker(vec![
        ("main.rs", false),
        ("lib.rs", false),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recent_picker(rp);
    let actual = render_to_string(40, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          ┌ Recent Files ────────────────────┐
          │>                                 │
          │main.rs                           │
          │lib.rs                            │
          │                                  │
          │                                  │
          │                                  │
          │                                  │
          │                                  │
         N└──────────────────────────────────┘1
        SPC for leader | : for commands | SPC q"#]]);
}
