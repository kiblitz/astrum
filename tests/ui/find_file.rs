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
    ff.path_selected = true; // Auto-selected when no matches.
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
          ~             │  ↑ Select path line and press Enter to create│
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

#[test]
fn find_file_many_entries() {
    let entries: Vec<_> = (0..20)
        .map(|i| (
            Box::leak(format!("file_{:02}.rs", i).into_boxed_str()) as &str,
            false,
            100u64,
        ))
        .collect();
    let ff = make_find_file("/test", entries);
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
          ~             │> /test\                                      │
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
fn find_file_selected_last() {
    let mut ff = make_find_file("/test", vec![
        ("a.rs", false, 10),
        ("b.rs", false, 20),
        ("c.rs", false, 30),
    ]);
    ff.selected = 2;
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
          ~             │> /test\                                      │
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
fn find_file_dirs_and_files() {
    let ff = make_find_file("/test", vec![
        ("src", true, 0),
        ("main.rs", false, 100),
        ("tests", true, 0),
        ("lib.rs", false, 200),
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
          ~             │> /test\                                      │
          ~             │src/                                          │
          ~             │main.rs                                       │
          ~             │tests/                                        │
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
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_with_input_and_entries() {
    let mut ff = make_find_file("/test", vec![
        ("src", true, 0),
        ("main.rs", false, 100),
        ("Cargo.toml", false, 200),
    ]);
    ff.input = "ma".to_string();
    ff.filtered = vec![1]; // main.rs matches
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
          ~             │> /test\ma                                    │
          ~             │main.rs                                       │
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
fn find_file_very_wide_terminal() {
    let ff = make_find_file("/test", vec![
        ("a.rs", false, 10),
        ("b.rs", false, 20),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff);
    let actual = render_to_string(160, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~                             ┌ Find File ───────────────────────────────────────────────────────────────────────────────────┐
          ~                             │> /test\                                                                                      │
          ~                             │a.rs                                                                                          │
          ~                             │b.rs                                                                                          │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             │                                                                                              │
          ~                             └──────────────────────────────────────────────────────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                                                                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_empty_dir() {
    let ff = make_find_file("/test", vec![]);
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
          ~             │> /test\                                      │
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
          ~             │                                              │
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn find_file_path_selected_highlight() {
    // When path_selected is true, the path line should be highlighted
    // and no entry should appear selected.
    let mut ff = make_find_file("/home/user/project", vec![
        (".", true, 0),
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    ff.input = "new_file.rs".to_string();
    ff.filtered = vec![]; // Nothing matches "new_file.rs"
    ff.path_selected = true;
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
          ~             │> /home/user/project\new_file.rs              │
          ~             │  ↑ Select path line and press Enter to create│
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
fn find_file_path_selected_with_matching_folder() {
    // User types a name that matches an existing folder. path_selected=true
    // means the path line is selected for file creation, not the folder entry.
    let mut ff = make_find_file("/test", vec![
        (".", true, 0),
        ("src", true, 0),
        ("tests", true, 0),
    ]);
    ff.input = "src".to_string();
    ff.filtered = vec![1]; // "src" folder matches
    ff.path_selected = true; // User pressed Up to select path line
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
          ~             │> /test\src                                   │
          ~             │src/                                          │
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
fn find_file_no_matches_path_auto_selected() {
    // When all entries are filtered out, path_selected is automatically set to true.
    // The create hint should appear and the path line should be highlighted.
    let mut ff = make_find_file("/test", vec![
        (".", true, 0),
        ("main.rs", false, 100),
    ]);
    ff.input = "brand_new.rs".to_string();
    ff.filtered = vec![]; // Nothing matches
    ff.path_selected = true; // Auto-set by refilter()
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
          ~             │> /test\brand_new.rs                          │
          ~             │  ↑ Select path line and press Enter to create│
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
fn find_file_path_selected_existing_name_shows_entries() {
    // When the user types a name matching an existing entry and selects the path line,
    // the matching entry should still be visible so the user sees the conflict.
    // The editor refuses to create the file if the path already exists.
    let mut ff = make_find_file("/test", vec![
        (".", true, 0),
        ("src", true, 0),
        ("main.rs", false, 100),
    ]);
    ff.input = "main.rs".to_string();
    ff.filtered = vec![2]; // "main.rs" matches
    ff.path_selected = true; // User pressed Up to select path line
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
          ~             │> /test\main.rs                               │
          ~             │main.rs                                       │
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
fn find_file_already_exists_error_with_overlay() {
    // After a failed creation attempt (path exists), the status message shows
    // the error while find-file remains open for the user to change the name.
    let mut ff = make_find_file("/test", vec![
        (".", true, 0),
        ("src", true, 0),
    ]);
    ff.input = "src".to_string();
    ff.filtered = vec![1]; // "src" folder matches
    ff.path_selected = true;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_find_file(ff)
        .with_status("\"src\" already exists");
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Find File ───────────────────────────────────┐
          ~             │> /test\src                                   │
          ~             │src/                                          │
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
        "src" already exists"#]]);
}
