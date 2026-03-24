use super::helpers::*;
use astrum::file_browser::FileBrowser;
use expect_test::expect;
use std::path::PathBuf;

fn make_test_browser(entries: Vec<(&str, bool, u64)>) -> FileBrowser {
    let mut fb = FileBrowser::new(PathBuf::from("/test/dir"));
    let dir_entries: Vec<_> = entries
        .into_iter()
        .map(|(name, is_dir, size)| make_dir_entry(name, is_dir, size))
        .collect();
    fb.set_entries(dir_entries);
    fb
}

#[test]
fn browser_empty_directory() {
    let fb = make_test_browser(vec![]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
          (empty directory)






         FILES  0/0  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_with_entries() {
    let fb = make_test_browser(vec![
        ("src", true, 0),
        ("tests", true, 0),
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  src/                                               dir
            tests/                                             dir
            Cargo.toml                                        1.2K
            README.md                                         567B



         FILES  1/4  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_selected_not_first() {
    let mut fb = make_test_browser(vec![
        ("src", true, 0),
        ("tests", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    fb.selected = 2;
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
            src/                                               dir
            tests/                                             dir
         >  Cargo.toml                                        1.2K




         FILES  3/3  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_large_file_sizes() {
    let fb = make_test_browser(vec![
        ("small.txt", false, 42),
        ("medium.dat", false, 15360),       // 15K
        ("large.bin", false, 2_500_000),    // ~2.4M
        ("huge.iso", false, 4_700_000_000), // ~4.4G
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  huge.iso                                          4.4G
            large.bin                                         2.4M
            medium.dat                                       15.0K
            small.txt                                          42B



         FILES  1/4  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_filtered() {
    let mut fb = make_test_browser(vec![
        ("src", true, 0),
        ("tests", true, 0),
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
    ]);
    fb.filter = "rs".to_string();
    fb.input_mode = astrum::file_browser::BrowserInputMode::Filter;
    // Manually refilter — only entries matching "rs" should appear.
    fb.filtered_indices = vec![]; // Will show entries based on filter
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
          (empty directory)






         FILTER  0/0  j/k:nav  enter:open  h/-:up  /:filter  n:new
        /rs"#]]);
}

#[test]
fn browser_narrow_width() {
    let fb = make_test_browser(vec![
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(30, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ──────────────────────────────
         >  src/                 dir
            Cargo.toml          1.2K



         FILES  1/2  j/k:nav  enter:op
        SPC for leader | : for command"#]]);
}

#[test]
fn browser_many_entries_scrolled() {
    let entries: Vec<_> = (0..20)
        .map(|i| (Box::leak(format!("file_{:02}.rs", i).into_boxed_str()) as &str, false, 100u64))
        .collect();
    let mut fb = make_test_browser(entries);
    fb.selected = 15;
    fb.scroll_offset = 10;
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
            file_10.rs                                        100B
            file_11.rs                                        100B
            file_12.rs                                        100B
            file_13.rs                                        100B
            file_14.rs                                        100B
         >  file_15.rs                                        100B
            file_16.rs                                        100B
         FILES  16/20  j/k:nav  enter:open  h/-:up  /:filter  n:new
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_new_file_mode() {
    use astrum::file_browser::BrowserInputMode;
    let mut fb = make_test_browser(vec![
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    fb.input_mode = BrowserInputMode::NewFile;
    fb.new_file_name = "new_module.rs".to_string();
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  src/                                               dir
            Cargo.toml                                        1.2K





         NEW FILE  1/2  j/k:nav  enter:open  h/-:up  /:filter  n:new
        New file: new_module.rs"#]]);
}

#[test]
fn browser_single_entry() {
    let fb = make_test_browser(vec![("README.md", false, 256)]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  README.md                                         256B




         FILES  1/1  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_only_dirs() {
    let fb = make_test_browser(vec![
        ("src", true, 0),
        ("tests", true, 0),
        ("docs", true, 0),
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  docs/                                              dir
            src/                                               dir
            tests/                                             dir


         FILES  1/3  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_only_files() {
    let fb = make_test_browser(vec![
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
        ("main.rs", false, 890),
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  Cargo.toml                                        1.2K
            main.rs                                           890B
            README.md                                         567B


         FILES  1/3  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_zero_byte_file() {
    let fb = make_test_browser(vec![
        ("empty.txt", false, 0),
        ("nonempty.txt", false, 42),
    ]);
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
         >  empty.txt                                           0B
            nonempty.txt                                       42B



         FILES  1/2  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_last_entry_selected() {
    let mut fb = make_test_browser(vec![
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
        ("README.md", false, 567),
    ]);
    fb.selected = 2;
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
         astrum
          /test/dir
        ────────────────────────────────────────────────────────────
            src/                                               dir
            Cargo.toml                                        1.2K
         >  README.md                                         567B


         FILES  3/3  j/k:nav  enter:open  h/-:up  /:filter  n:new  ~
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn browser_with_buffer_in_split() {
    use astrum::pane::SplitDirection;
    let buf = make_buffer("hello\n", "main.rs");
    let fb = make_test_browser(vec![
        ("src", true, 0),
        ("Cargo.toml", false, 1234),
    ]);
    let mut state = RenderState::default().with_buffer(buf);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    let state = state.with_file_browser(p2, fb);
    let actual = render_to_string(80, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello                              │  /test/dir
          ~                                     │───────────────────────────────────────
          ~                                     │ >  src/                          dir
          ~                                     │    Cargo.toml                   1.2K
          ~                                     │
          ~                                     │
          ~                                     │
          ~                                     │
         main.rs                                │ main.rs
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}
