use crate::helpers::*;
use astrum::input::Mode;
use astrum::pane::{PaneContent, SplitDirection};
use expect_test::expect;

#[test]
fn terminal_pane_shows_grid_content() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &[
        "$ echo hello",
        "hello",
        "$ ",
    ], "[terminal]");
    let state = state.with_terminal(pane_id, term);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $ echo hello
        hello
        $


















         TERMINAL   [terminal]
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_status_line_insert_mode() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ "], "bash");
    let state = state.with_terminal(pane_id, term).with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $




















         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_status_line_normal_mode() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ "], "bash");
    let state = state.with_terminal(pane_id, term).with_mode(Mode::Normal);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $




















         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_exited_shows_message() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let mut term = make_terminal(20, 80, &["$ exit", ""], "[terminal]");
    term.exited = Some("Process exited (0)".to_string());
    let state = state.with_terminal(pane_id, term);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        Process exited (0)




















         TERMINAL   [terminal]                                       Process exited (0)
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_with_title() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["running..."], "my-shell");
    let state = state.with_terminal(pane_id, term).with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        running...




















         TERMINAL   my-shell
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_left_buffer_right_split() {
    let mut state = RenderState::default();
    let p1 = state.pane_layout.active_id;
    let term = make_terminal(20, 40, &["$ ls", "file.rs", "$ "], "bash");
    state.pane_layout.pane_by_id_mut(p1).unwrap().content = PaneContent::Terminal(term);

    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    let buf = make_buffer("fn main() {}\n", "main.rs");
    let buf_id = buf.id;
    state.buffers.push(buf);
    state.pane_layout.pane_by_id_mut(p2).unwrap().content = PaneContent::Buffer(buf_id);

    // Focus on terminal pane.
    state.pane_layout.active_id = p1;
    let state = state.with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
        $ ls                                    │  1  fn main() {}
        file.rs                                 │  ~
        $                                       │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
                                                │  ~
         bash                                   │ main.rs
         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_top_buffer_bottom_split() {
    let mut state = RenderState::default();
    let p1 = state.pane_layout.active_id;
    let term = make_terminal(10, 80, &["$ cargo build", "Compiling...", "$ "], "powershell");
    state.pane_layout.pane_by_id_mut(p1).unwrap().content = PaneContent::Terminal(term);

    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    let buf = make_buffer("hello world\n", "test.txt");
    let buf_id = buf.id;
    state.buffers.push(buf);
    state.pane_layout.pane_by_id_mut(p2).unwrap().content = PaneContent::Buffer(buf_id);

    state.pane_layout.active_id = p1;
    let state = state.with_mode(Mode::Normal);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          test.txt
        $ cargo build
        Compiling...
        $






         powershell
        ────────────────────────────────────────────────────────────────────────────────
          1  hello world
          ~
          ~
          ~
          ~
          ~
          ~
          ~
          ~
         test.txt
         TERMINAL   powershell
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_with_find_file_overlay() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ ", ""], "bash");
    state.pane_layout.pane_by_id_mut(pane_id).unwrap().content = PaneContent::Terminal(term);

    let ff = make_find_file("/home/user/project", vec![
        ("src", true, 0),
        ("main.rs", false, 1024),
        ("Cargo.toml", false, 512),
    ]);
    let state = state.with_find_file(ff).with_mode(Mode::Normal);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $


                        ┌ Find File ───────────────────────────────────┐
                        │> /home/user/project\                         │
                        │src/                                          │
                        │main.rs                                       │
                        │Cargo.toml                                    │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        │                                              │
                        └──────────────────────────────────────────────┘


         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_with_command_mode() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ whoami", "user", "$ "], "bash");
    state.pane_layout.pane_by_id_mut(pane_id).unwrap().content = PaneContent::Terminal(term);

    let state = state.with_command("w");
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $ whoami
        user
        $


















         TERMINAL   bash
        :w"#]]);
}

#[test]
fn terminal_with_status_message() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ "], "bash");
    state.pane_layout.pane_by_id_mut(pane_id).unwrap().content = PaneContent::Terminal(term);

    let state = state.with_status("Terminal opened").with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $




















         TERMINAL   bash
        Terminal opened"#]]);
}

#[test]
fn two_terminal_panes_split() {
    let mut state = RenderState::default();
    let p1 = state.pane_layout.active_id;
    let term1 = make_terminal(20, 40, &["$ echo left", "left"], "bash");
    state.pane_layout.pane_by_id_mut(p1).unwrap().content = PaneContent::Terminal(term1);

    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    let term2 = make_terminal(20, 40, &["$ echo right", "right"], "zsh");
    state.pane_layout.pane_by_id_mut(p2).unwrap().content = PaneContent::Terminal(term2);

    state.pane_layout.active_id = p1;
    let state = state.with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $ echo left                             │$ echo right
        left                                    │right
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
                                                │
         bash                                   │ zsh
         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_multiline_output() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &[
        "$ cat /etc/passwd",
        "root:x:0:0:root:/root:/bin/bash",
        "daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin",
        "bin:x:2:2:bin:/bin:/usr/sbin/nologin",
        "sys:x:3:3:sys:/dev:/usr/sbin/nologin",
        "$ ",
    ], "bash");
    let state = state.with_terminal(pane_id, term).with_mode(Mode::Insert);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $ cat /etc/passwd
        root:x:0:0:root:/root:/bin/bash
        daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
        bin:x:2:2:bin:/bin:/usr/sbin/nologin
        sys:x:3:3:sys:/dev:/usr/sbin/nologin
        $















         TERMINAL   bash
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn terminal_recording_macro_indicator() {
    let mut state = RenderState::default();
    let pane_id = state.pane_layout.active_id;
    let term = make_terminal(20, 80, &["$ "], "bash");
    state.pane_layout.pane_by_id_mut(pane_id).unwrap().content = PaneContent::Terminal(term);

    let state = state.with_recording_macro('a').with_mode(Mode::Normal);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum
        $




















         TERMINAL   bash
        recording @a SPC for leader | : for commands | SPC q q to quit"#]]);
}
