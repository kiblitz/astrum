use crate::helpers::*;
use astrum::input::Mode;
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
