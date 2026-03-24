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
