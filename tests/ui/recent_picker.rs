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

#[test]
fn recent_single_item() {
    let rp = make_recent_picker(vec![("src/main.rs", false)]);
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
fn recent_all_dirs() {
    let rp = make_recent_picker(vec![
        ("src", true),
        ("tests", true),
        ("docs", true),
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
          ~             │docs/                                         │
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
fn recent_no_matches_after_filter() {
    let mut rp = make_recent_picker(vec![
        ("main.rs", false),
        ("lib.rs", false),
    ]);
    rp.query = "zzz".to_string();
    rp.filtered = vec![];
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
          ~             │> zzz                                         │
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
fn recent_many_items() {
    let items: Vec<_> = (0..20)
        .map(|i| (
            Box::leak(format!("file_{:02}.rs", i).into_boxed_str()) as &str,
            false,
        ))
        .collect();
    let rp = make_recent_picker(items);
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
          ~             │file_00.rs                                    │
          ~             │file_01.rs                                    │
          ~             │file_02.rs                                    │
          ~             │file_03.rs                                    │
          ~             │file_04.rs                                    │
          ~             │file_05.rs                                    │
          ~             │file_06.rs                                    │
          ~             │file_07.rs                                    │
          ~             │file_08.rs                                    │
          ~             │file_09.rs                                    │
          ~             │file_10.rs                                    │
          ~             │file_11.rs                                    │
          ~             │file_12.rs                                    │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn recent_selected_last() {
    let mut rp = make_recent_picker(vec![
        ("a.rs", false),
        ("b.rs", false),
        ("c.rs", false),
    ]);
    rp.selected = 2;
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
          ~             │a.rs                                          │
          ~             │b.rs                                          │
          ~             │c.rs                                          │
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
fn recent_long_paths() {
    let rp = make_recent_picker(vec![
        ("src/very/deeply/nested/directory/structure/main.rs", false),
        ("another/long/path/to/some/module/lib.rs", false),
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
          ~             │src/very/deeply/nested/directory/structure/mai│
          ~             │another/long/path/to/some/module/lib.rs       │
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
fn recent_mixed_long_short() {
    let rp = make_recent_picker(vec![
        ("a.rs", false),
        ("src/deeply/nested/module.rs", false),
        ("b.rs", false),
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
          ~             │a.rs                                          │
          ~             │src/deeply/nested/module.rs                   │
          ~             │b.rs                                          │
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
