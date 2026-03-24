use super::helpers::*;
use astrum::pane::SplitDirection;
use expect_test::expect;

#[test]
fn single_pane_full_width() {
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(60, 12, &state);
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
         NORMAL  main.rs                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn vertical_split_two_panes() {
    let buf1 = make_buffer("left pane\n", "left.rs");
    let buf2 = make_buffer("right pane\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let new_pane = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(new_pane) {
        pane.buffer_id = Some(buf2_id);
    }
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  left pane                │  1  right pane
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         left.rs                      │ right.rs
         NORMAL  left.rs                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn horizontal_split_two_panes() {
    let buf1 = make_buffer("top pane\n", "top.rs");
    let buf2 = make_buffer("bottom pane\n", "bottom.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let new_pane = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(new_pane) {
        pane.buffer_id = Some(buf2_id);
    }
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          top.rs  │  bottom.rs
          1  top pane
          ~
          ~
         top.rs
        ────────────────────────────────────────────────────────────
          1  bottom pane
          ~
          ~
         bottom.rs
         NORMAL  top.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn vertical_split_active_indicator() {
    let buf1 = make_buffer("left\n", "left.rs");
    let buf2 = make_buffer("right\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let new_pane = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(new_pane) {
        pane.buffer_id = Some(buf2_id);
    }
    // Active pane is still the first one.
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  left                     │  1  right
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         left.rs                      │ right.rs
         NORMAL  left.rs                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn three_way_vertical_split() {
    let buf1 = make_buffer("one\n", "a.rs");
    let buf2 = make_buffer("two\n", "b.rs");
    let buf3 = make_buffer("three\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.buffer_id = Some(buf2_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.buffer_id = Some(buf3_id);
    }
    let actual = render_to_string(80, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  one                  │  1  three                │  1  two
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
         a.rs                     │ c.rs                     │ b.rs
         NORMAL  a.rs                                                               1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn pane_without_buffer() {
    let buf = make_buffer("hello\n", "main.rs");
    let mut state = RenderState::default()
        .with_buffer(buf);
    // Split creates new pane with no buffer.
    state.pane_layout.split(SplitDirection::Vertical);
    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello                    │  1  hello
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         main.rs                      │ main.rs
         NORMAL  main.rs                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn nested_splits() {
    // Vertical split, then horizontal split on right pane.
    let buf1 = make_buffer("left\n", "left.rs");
    let buf2 = make_buffer("top-right\n", "tr.rs");
    let buf3 = make_buffer("bot-right\n", "br.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.buffer_id = Some(buf2_id);
    }
    // Focus p2, then split horizontally.
    state.pane_layout.active_id = p2;
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.buffer_id = Some(buf3_id);
    }
    let actual = render_to_string(80, 16, &state);
    check(&actual, expect![[r#"
          left.rs  │  tr.rs  │  br.rs
          1  left                               │  1  top-right
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │ tr.rs
          ~                                     │───────────────────────────────────────
          ~                                     │  1  bot-right
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
         left.rs                                │ br.rs
         NORMAL  tr.rs                                                              1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}
