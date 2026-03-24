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
