use super::helpers::*;
use astrum::action::SearchDirection;
use expect_test::expect;

#[test]
fn cmdline_normal_default_help() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
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
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn cmdline_command_mode_prefix() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("w myfile.rs");
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
        :w myfile.rs"#]]);
}

#[test]
fn cmdline_command_mode_empty() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("");
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
        :"#]]);
}

#[test]
fn cmdline_search_forward() {
    let buf = make_buffer("hello world\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("world", SearchDirection::Forward);
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
         SEARCH  main.rs                                                            1:1
        /world"#]]);
}

#[test]
fn cmdline_search_backward() {
    let buf = make_buffer("hello world\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("hello", SearchDirection::Backward);
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
         SEARCH  main.rs                                                            1:1
        ?hello"#]]);
}

#[test]
fn cmdline_status_message() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_status("File saved successfully");
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
         NORMAL  main.rs                                                            1:1
        File saved successfully"#]]);
}

#[test]
fn cmdline_which_key_hints() {
    let buf = make_buffer("hello\n", "main.rs");
    let hints = vec![
        ("f".to_string(), "file".to_string()),
        ("b".to_string(), "buffer".to_string()),
        ("q".to_string(), "quit".to_string()),
    ];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_pending_keys("SPC", hints);
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
         NORMAL  main.rs                                                            1:1
        SPC-  f:file b:buffer q:quit"#]]);
}

#[test]
fn cmdline_which_key_many_hints() {
    let buf = make_buffer("hello\n", "main.rs");
    let hints: Vec<(String, String)> = (b'a'..=b'z')
        .map(|c| (format!("{}", c as char), format!("action_{}", c as char)))
        .collect();
    let state = RenderState::default()
        .with_buffer(buf)
        .with_pending_keys("SPC", hints);
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
         NORMAL  main.rs                                                            1:1
        SPC-  a:action_a b:action_b c:action_c d:action_d e:action_e f:action_f"#]]);
}

#[test]
fn cmdline_recording_macro() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recording_macro('a');
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
         NORMAL  main.rs                                                            1:1
        recording @a SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn cmdline_recording_plus_status() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recording_macro('q')
        .with_status("Pattern not found");
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
         NORMAL  main.rs                                                            1:1
        recording @q Pattern not found"#]]);
}

#[test]
fn cmdline_very_long_command() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_command("s/very_long_pattern_that_extends_past_normal_width/replacement_string/g");
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         COMMAND  main.rs                                                           1:1
        :s/very_long_pattern_that_extends_past_normal_width/replacement_string/g"#]]);
}

#[test]
fn cmdline_which_key_single_hint() {
    let buf = make_buffer("hello\n", "main.rs");
    let hints = vec![("q".to_string(), "quit".to_string())];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_pending_keys("SPC", hints);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC-  q:quit"#]]);
}

#[test]
fn cmdline_which_key_narrow() {
    let buf = make_buffer("hello\n", "main.rs");
    let hints = vec![
        ("f".to_string(), "file".to_string()),
        ("b".to_string(), "buffer".to_string()),
        ("q".to_string(), "quit".to_string()),
    ];
    let state = RenderState::default()
        .with_buffer(buf)
        .with_pending_keys("SPC", hints);
    let actual = render_to_string(30, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         NORMAL  main.rs          1:1
        SPC-  f:file b:buffer q:quit"#]]);
}

#[test]
fn cmdline_recording_with_command() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recording_macro('z')
        .with_command("wq");
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         COMMAND  main.rs                                                           1:1
        recording @z :wq"#]]);
}

#[test]
fn cmdline_recording_with_search() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_recording_macro('a')
        .with_search("pattern", SearchDirection::Forward);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         SEARCH  main.rs                                                            1:1
        recording @a /pattern"#]]);
}

#[test]
fn cmdline_browser_filter_mode() {
    use astrum::file_browser::{FileBrowser, BrowserInputMode};
    use std::path::PathBuf;
    let mut fb = FileBrowser::new(PathBuf::from("/test"));
    fb.filter = "toml".to_string();
    fb.input_mode = BrowserInputMode::Filter;
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 8, &state);
    check(&actual, expect![[r#"
         astrum
          /test
        ────────────────────────────────────────────────────────────
          (empty directory)


         FILTER  0/0  j/k:nav  enter:open  h/-:up  /:filter  n:new
        /toml"#]]);
}

#[test]
fn cmdline_browser_new_file_mode() {
    use astrum::file_browser::{FileBrowser, BrowserInputMode};
    use std::path::PathBuf;
    let mut fb = FileBrowser::new(PathBuf::from("/test"));
    fb.new_file_name = "new_module.rs".to_string();
    fb.input_mode = BrowserInputMode::NewFile;
    let state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let state = state.with_file_browser(pane_id, fb);
    let actual = render_to_string(60, 8, &state);
    check(&actual, expect![[r#"
         astrum
          /test
        ────────────────────────────────────────────────────────────
          (empty directory)


         NEW FILE  0/0  j/k:nav  enter:open  h/-:up  /:filter  n:new
        New file: new_module.rs"#]]);
}

#[test]
fn cmdline_search_empty_buffer() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_search("", SearchDirection::Forward);
    let actual = render_to_string(80, 8, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~
          ~
         SEARCH  main.rs                                                            1:1
        /"#]]);
}
