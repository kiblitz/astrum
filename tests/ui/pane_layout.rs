use super::helpers::*;
use astrum::pane::{FocusDirection, PaneContent, SplitDirection};
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
        pane.content = PaneContent::Buffer(buf2_id);
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
        pane.content = PaneContent::Buffer(buf2_id);
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
        pane.content = PaneContent::Buffer(buf2_id);
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
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
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
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus p2, then split horizontally.
    state.pane_layout.active_id = p2;
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
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

#[test]
fn very_narrow_vertical_split() {
    let buf1 = make_buffer("a\n", "a.rs");
    let buf2 = make_buffer("b\n", "b.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let actual = render_to_string(24, 8, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs
          1  a      │  1  b
          ~         │  ~
          ~         │  ~
          ~         │  ~
         a.rs       │ b.rs
         NORMAL  a.rs       1:1
        SPC for leader | : for c"#]]);
}

#[test]
fn very_short_horizontal_split() {
    let buf1 = make_buffer("top\n", "top.rs");
    let buf2 = make_buffer("bot\n", "bot.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let actual = render_to_string(40, 8, &state);
    check(&actual, expect![[r#"
          top.rs  │  bot.rs
          1  top
         top.rs
        ────────────────────────────────────────
          1  bot
         bot.rs
         NORMAL  top.rs                     1:1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn four_way_split() {
    let buf1 = make_buffer("1\n", "a.rs");
    let buf2 = make_buffer("2\n", "b.rs");
    let buf3 = make_buffer("3\n", "c.rs");
    let buf4 = make_buffer("4\n", "d.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let buf4_id = buf4.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3)
        .with_extra_buffer(buf4);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Split first pane horizontally
    state.pane_layout.active_id = state.pane_layout.panes[0].id;
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Split second pane horizontally
    state.pane_layout.active_id = p2;
    let p4 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p4) {
        pane.content = PaneContent::Buffer(buf4_id);
    }
    let actual = render_to_string(80, 16, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs  │  d.rs
          1  1                                  │  1  2
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
         a.rs                                   │ b.rs
        ────────────────────────────────────────│───────────────────────────────────────
          1  3                                  │  1  4
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
          ~                                     │  ~
         c.rs                                   │ d.rs
         NORMAL  b.rs                                                               1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn split_with_modified_buffers() {
    let mut buf1 = make_buffer("left\n", "left.rs");
    buf1.modified = true;
    let buf2 = make_buffer("right\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
          left.rs [+]  │  right.rs
          1  left                     │  1  right
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         left.rs                      │ right.rs
         NORMAL  left.rs [+]                                    1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn split_active_right_pane() {
    let buf1 = make_buffer("left\n", "left.rs");
    let buf2 = make_buffer("right\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Make the right pane active
    state.pane_layout.active_id = p2;
    let actual = render_to_string(60, 10, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  left                     │  1  right
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         left.rs                      │ right.rs
         NORMAL  right.rs                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn three_horizontal_splits() {
    let buf1 = make_buffer("one\n", "a.rs");
    let buf2 = make_buffer("two\n", "b.rs");
    let buf3 = make_buffer("three\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  one
          ~
          ~
         a.rs
        ────────────────────────────────────────────────────────────
          1  three
          ~
          ~
         c.rs
        ────────────────────────────────────────────────────────────
          1  two
          ~
         b.rs
         NORMAL  a.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_bottom_pane_right_in_horizontal_split() {
    // Layout: Horizontal { [Vertical { [top-left, top-right] }, bottom] }
    // Move bottom pane right → expected: bottom pane becomes rightmost vertical pane
    // i.e. Vertical { [Horizontal-or-single { top-left, top-right }, bottom] }
    //
    // The bug: bottom pane ends up between top-left and top-right instead of
    // at the far right of the layout.
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    // Split active pane horizontally: top and bottom
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Split top pane vertically: top-left and top-right
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus the bottom pane and move it right
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(80, 16, &state);
    // Expected: top-left and top-right on the left, bottom on the far right
    // NOT: top-left, bottom, top-right (the bug)
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  TL                   │  1  TR                   │  1  BOT
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
         tl.rs                    │ tr.rs                    │ bot.rs
         NORMAL  bot.rs                                                             1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_bottom_pane_left_in_horizontal_split() {
    // Same layout as above, but move bottom pane LEFT.
    // Expected: bottom pane becomes leftmost vertical pane.
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(80, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  BOT                  │  1  TL                   │  1  TR
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
          ~                       │  ~                       │  ~
         bot.rs                   │ tl.rs                    │ tr.rs
         NORMAL  bot.rs                                                             1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_right_pane_down_in_vertical_split() {
    // Symmetric case: Vertical { [Horizontal { [left-top, left-bot] }, right] }
    // Move right pane down.
    let buf1 = make_buffer("LT\n", "lt.rs");
    let buf2 = make_buffer("LB\n", "lb.rs");
    let buf3 = make_buffer("RT\n", "rt.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          lt.rs  │  lb.rs  │  rt.rs
          1  LT
          ~
          ~
         lt.rs
        ────────────────────────────────────────────────────────────
          1  LB
          ~
          ~
         lb.rs
        ────────────────────────────────────────────────────────────
          1  RT
          ~
         rt.rs
         NORMAL  rt.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

// ─── Comprehensive pane movement tests ───────────────────────────────────

#[test]
fn move_top_right_pane_down_in_nested_layout() {
    // Layout: Horizontal { Vertical { [TL, TR] }, BOT }
    // Move TR down → expect: TL on top, BOT in middle, TR at bottom
    // (TR should go to the END of the outer horizontal container)
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    // Split horizontally first: top and bottom
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Split top vertically: TL and TR
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus TR and move it down
    state.pane_layout.active_id = p3;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  TL
          ~
          ~
         tl.rs
        ────────────────────────────────────────────────────────────
          1  BOT
          ~
          ~
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TR
          ~
         tr.rs
         NORMAL  tr.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_top_left_pane_down_in_nested_layout() {
    // Layout: Horizontal { Vertical { [TL, TR] }, BOT }
    // Move TL down → expect: TR on top, BOT in middle, TL at bottom
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    // Focus is on TL (the original active pane before splits)
    // Actually after splitting, the active is still the original. Let me get the first pane.
    // After split_horizontal: p1 (top), p2 (bottom=buf3)
    // After split_vertical on p1: p1 (left=TL), p3 (right=buf2=TR)
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  TR
          ~
          ~
         tr.rs
        ────────────────────────────────────────────────────────────
          1  BOT
          ~
          ~
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TL
          ~
         tl.rs
         NORMAL  tl.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_bottom_pane_up_in_nested_layout() {
    // Layout: Horizontal { Vertical { [TL, TR] }, BOT }
    // Move BOT up → expect: BOT on top, TL and TR side-by-side on bottom
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Up);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  BOT
          ~
          ~
          ~
          ~
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TL                       │  1  TR
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         tl.rs                        │ tr.rs
         NORMAL  bot.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_left_pane_right_in_nested_layout() {
    // Layout: Vertical { Horizontal { [LT, LB] }, RT }
    // Move LT right → expect: LB on left, RT in middle, LT at right
    let buf1 = make_buffer("LT\n", "lt.rs");
    let buf2 = make_buffer("LB\n", "lb.rs");
    let buf3 = make_buffer("RT\n", "rt.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          lt.rs  │  lb.rs  │  rt.rs
          1  LB             │  1  RT            │  1  LT
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         lb.rs              │ rt.rs             │ lt.rs
         NORMAL  lt.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_down_simple_horizontal_split() {
    // Simple: Horizontal { [top, bottom] }
    // Move top down → becomes: Horizontal { [bottom, top] }
    let buf1 = make_buffer("TOP\n", "top.rs");
    let buf2 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          top.rs  │  bot.rs
          1  BOT
          ~
          ~
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TOP
          ~
          ~
         top.rs
         NORMAL  top.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_up_simple_horizontal_split() {
    // Simple: Horizontal { [top, bottom] }
    // Move bottom up → becomes: Horizontal { [bottom, top] }
    let buf1 = make_buffer("TOP\n", "top.rs");
    let buf2 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Up);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          top.rs  │  bot.rs
          1  BOT
          ~
          ~
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TOP
          ~
          ~
         top.rs
         NORMAL  bot.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_right_simple_vertical_split() {
    // Simple: Vertical { [left, right] }
    // Move left right → becomes: Vertical { [right, left] }
    let buf1 = make_buffer("LEFT\n", "left.rs");
    let buf2 = make_buffer("RIGHT\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  RIGHT                    │  1  LEFT
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         right.rs                     │ left.rs
         NORMAL  left.rs                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_left_simple_vertical_split() {
    // Simple: Vertical { [left, right] }
    // Move right left → becomes: Vertical { [right, left] }
    let buf1 = make_buffer("LEFT\n", "left.rs");
    let buf2 = make_buffer("RIGHT\n", "right.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          left.rs  │  right.rs
          1  RIGHT                    │  1  LEFT
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         right.rs                     │ left.rs
         NORMAL  right.rs                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_in_three_way_vertical_left_to_right() {
    // Vertical { [A, B, C] }
    // Move A right → Vertical { [B, C, A] }
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Focus A (first pane) and move right
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  C              │  1  B             │  1  A
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         c.rs               │ b.rs              │ a.rs
         NORMAL  a.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_in_three_way_vertical_right_to_left() {
    // Vertical { [A, B, C] }
    // Move C left → Vertical { [C, A, B] }
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p2;
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Focus C (last pane) and move left
    state.pane_layout.active_id = p3;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  C              │  1  A             │  1  B
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         c.rs               │ a.rs              │ b.rs
         NORMAL  c.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_middle_pane_right_in_three_way() {
    // Vertical { [A, B, C] }
    // Move B right → Vertical { [A, C, B] }
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p2;
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  A              │  1  C             │  1  B
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         a.rs               │ c.rs              │ b.rs
         NORMAL  b.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_middle_pane_left_in_three_way() {
    // Vertical { [A, B, C] }
    // Move B left → Vertical { [B, A, C] }
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p2;
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    state.pane_layout.active_id = p2;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs
          1  B              │  1  A             │  1  C
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         b.rs               │ a.rs              │ c.rs
         NORMAL  b.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_top_right_pane_left_in_nested_layout() {
    // Layout: Horizontal { Vertical { [TL, TR] }, BOT }
    // Move TR left → expect: TR becomes leftmost: Horizontal { Vertical { [TR, TL] }, BOT }
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p3;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  bot.rs
          1  TR                       │  1  TL
          ~                           │  ~
          ~                           │  ~
         tr.rs                        │ tl.rs
        ────────────────────────────────────────────────────────────
          1  BOT
          ~
          ~
         bot.rs
         NORMAL  tr.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_down_across_axis_four_panes() {
    // Layout: Horizontal { Vertical { [A, B] }, Vertical { [C, D] } }
    // (2x2 grid: A|B on top, C|D on bottom)
    // Move B down → expect B at bottom-right or bottom of the layout
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf4 = make_buffer("D\n", "d.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let buf4_id = buf4.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3)
        .with_extra_buffer(buf4);
    // Create top/bottom horizontal split
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    // Split top into A|B
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus bottom pane
    state.pane_layout.active_id = p2;
    // Split bottom into C|D
    let p4 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p4) {
        pane.content = PaneContent::Buffer(buf4_id);
    }
    // Move B down
    state.pane_layout.active_id = p3;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs  │  d.rs
          1  A
          ~
          ~
         a.rs
        ────────────────────────────────────────────────────────────
          1  C                        │  1  D
          ~                           │  ~
          ~                           │  ~
         c.rs                         │ d.rs
        ────────────────────────────────────────────────────────────
          1  B
          ~
         b.rs
         NORMAL  b.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_top_right_down_in_three_row_layout() {
    // Layout: Horizontal { Vertical { [TL, TR] }, MID, BOT }
    // Move TR down → expect TR at the bottom (after BOT), not between MID and BOT
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("TR\n", "tr.rs");
    let buf3 = make_buffer("MID\n", "mid.rs");
    let buf4 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let buf4_id = buf4.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3)
        .with_extra_buffer(buf4);
    // Create: Horizontal { [p1(TL), p2, p4] }
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Split p2 (second row) to add a third row
    state.pane_layout.active_id = p2;
    let p4 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p4) {
        pane.content = PaneContent::Buffer(buf4_id);
    }
    // Now split top pane (p1) vertically: TL and TR
    let p1 = state.pane_layout.panes[0].id; // original first pane
    state.pane_layout.active_id = p1;
    let p3 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Layout should be: Horizontal { Vertical{TL, TR}, MID, BOT }
    // Focus TR and move it down
    state.pane_layout.active_id = p3;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  tr.rs  │  mid.rs  │  bot.rs
          1  TL
          ~
         tl.rs
        ────────────────────────────────────────────────────────────
          1  MID
          ~
         mid.rs
        ────────────────────────────────────────────────────────────
          1  BOT
         bot.rs
        ────────────────────────────────────────────────────────────
          1  TR
         tr.rs
         NORMAL  tr.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_nested_pane_down_in_vertical_root() {
    // Layout: Vertical { Horizontal { [TL, BL] }, RIGHT }
    // (Left column has top/bottom, right column is single pane)
    // Move TL down → expect TL at the bottom of the left column: Vertical { Horizontal{BL, TL}, RIGHT }
    // NOT: TL appearing below the entire layout as a full-width row
    let buf1 = make_buffer("TL\n", "tl.rs");
    let buf2 = make_buffer("BL\n", "bl.rs");
    let buf3 = make_buffer("RT\n", "rt.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    // Vertical split: left and right
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // Horizontal split of left: TL and BL
    let p3 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p3) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Layout: Vertical { Horizontal{TL(p1), BL(p3)}, RT(p2) }
    let p1 = state.pane_layout.active_id;
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          tl.rs  │  bl.rs  │  rt.rs
          1  BL                       │  1  RT
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         bl.rs                        │  ~
        ──────────────────────────────│  ~
          1  TL                       │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         tl.rs                        │ rt.rs
         NORMAL  tl.rs                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane2_down_in_horizontal_vertical_3() {
    // Exact user repro: Horizontal { Vertical { [1, 2] }, 3 }
    // Created by: split horizontal, then split vertical on top pane
    // Move pane 2 down → expect: 1(top), 3(middle), 2(bottom)
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    // active = pane1. Split horizontal → Horizontal{pane1, pane_h}
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    // active = pane1. Split vertical on pane1 → Horizontal{Vertical{pane1, pane_v}, pane_h}
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus pane2 (the right pane in the vertical split) and move down
    state.pane_layout.active_id = pane_v;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  ONE
          ~
          ~
         one.rs
        ────────────────────────────────────────────────────────────
          1  THREE
          ~
          ~
         three.rs
        ────────────────────────────────────────────────────────────
          1  TWO
          ~
         two.rs
         NORMAL  two.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane1_down_in_horizontal_vertical_3() {
    // Horizontal { Vertical { [1, 2] }, 3 }
    // Move pane 1 down → expect: 2(top), 3(middle), 1(bottom)
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // Focus pane1 (the left pane in the vertical split) and move down
    let pane1 = state.pane_layout.active_id;
    state.pane_layout.active_id = pane1;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  TWO
          ~
          ~
         two.rs
        ────────────────────────────────────────────────────────────
          1  THREE
          ~
          ~
         three.rs
        ────────────────────────────────────────────────────────────
          1  ONE
          ~
         one.rs
         NORMAL  one.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane3_up_in_horizontal_vertical_3() {
    // Horizontal { Vertical { [1, 2] }, 3 }
    // Move pane 3 up → expect: 3(top), 1|2(bottom side-by-side)
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = pane_h;
    let result = state.pane_layout.move_direction(FocusDirection::Up);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  THREE
          ~
          ~
          ~
          ~
         three.rs
        ────────────────────────────────────────────────────────────
          1  ONE                      │  1  TWO
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         one.rs                       │ two.rs
         NORMAL  three.rs                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane2_right_in_horizontal_vertical_3() {
    // Horizontal { Vertical { [1, 2] }, 3 }
    // Move pane 2 right → should stay within same row (swap within vertical)
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    // pane2 is already rightmost in its Vertical split — moving right should do nothing or wrap
    state.pane_layout.active_id = pane_v;
    state.pane_layout.move_direction(FocusDirection::Right);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  ONE                      │  1  TWO
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         one.rs                       │  ~
        ──────────────────────────────│  ~
          1  THREE                    │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         three.rs                     │ two.rs
         NORMAL  two.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane2_left_in_horizontal_vertical_3() {
    // Horizontal { Vertical { [1, 2] }, 3 }
    // Move pane 2 left → Horizontal { Vertical { [2, 1] }, 3 }
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = pane_v;
    state.pane_layout.move_direction(FocusDirection::Left);

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  TWO                      │  1  ONE
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         two.rs                       │ one.rs
        ────────────────────────────────────────────────────────────
          1  THREE
          ~
          ~
          ~
          ~
         three.rs
         NORMAL  two.rs                                         1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_across_axis_then_back() {
    // Layout: Vertical { [A, B] }
    // Move A down → Horizontal { [B, A] } (cross-axis wrap)... actually this should
    // just swap since they're on the same axis after target_dir computation.
    // Wait — Vertical split = side by side. Moving A down = target Horizontal.
    // That's opposite axis. Result: Horizontal { Vertical{...}, A } ... let's see.
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let p1 = state.pane_layout.active_id;
    // Move A (left pane) down — crosses axis
    state.pane_layout.active_id = p1;
    state.pane_layout.move_direction(FocusDirection::Down);

    let actual = render_to_string(60, 12, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs
          1  B
          ~
          ~
         b.rs
        ────────────────────────────────────────────────────────────
          1  A
          ~
          ~
         a.rs
         NORMAL  a.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_down_vertical_first_then_horizontal() {
    // Split order: vertical first, then horizontal (reverse of normal)
    // Creates: Vertical { Horizontal { [1, 3] }, 2 }
    // Move pane 3 (bottom-left) right → should go to rightmost column
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    state.pane_layout.active_id = pane_h;
    let result = state.pane_layout.move_direction(FocusDirection::Right);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  ONE            │  1  TWO           │  1  THREE
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
          ~                 │  ~                │  ~
         one.rs             │ two.rs            │ three.rs
         NORMAL  three.rs                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_up_vertical_first_then_horizontal() {
    // Vertical { Horizontal { [1, 3] }, 2 }
    // Move pane 3 (bottom-left) up → swap within same column
    let buf1 = make_buffer("ONE\n", "one.rs");
    let buf2 = make_buffer("TWO\n", "two.rs");
    let buf3 = make_buffer("THREE\n", "three.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let pane_v = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_v) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let pane_h = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(pane_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    state.pane_layout.active_id = pane_h;
    let result = state.pane_layout.move_direction(FocusDirection::Up);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          one.rs  │  two.rs  │  three.rs
          1  THREE                    │  1  TWO
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         three.rs                     │  ~
        ──────────────────────────────│  ~
          1  ONE                      │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
          ~                           │  ~
         one.rs                       │ two.rs
         NORMAL  three.rs                                       1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_deeply_nested_pane_down() {
    // 4-pane: Horizontal { Vertical { [A, B] }, Vertical { [C, D] } }
    // Move B down → expect B at the bottom
    let buf1 = make_buffer("A\n", "a.rs");
    let buf2 = make_buffer("B\n", "b.rs");
    let buf3 = make_buffer("C\n", "c.rs");
    let buf4 = make_buffer("D\n", "d.rs");
    let buf2_id = buf2.id;
    let buf3_id = buf3.id;
    let buf4_id = buf4.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3)
        .with_extra_buffer(buf4);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p_v1 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p_v1) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    state.pane_layout.active_id = p_h;
    let p_v2 = state.pane_layout.split(SplitDirection::Vertical);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p_h) {
        pane.content = PaneContent::Buffer(buf3_id);
    }
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p_v2) {
        pane.content = PaneContent::Buffer(buf4_id);
    }
    state.pane_layout.active_id = p_v1;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move_direction should return true");

    let actual = render_to_string(60, 16, &state);
    check(&actual, expect![[r#"
          a.rs  │  b.rs  │  c.rs  │  d.rs
          1  A
          ~
          ~
         a.rs
        ────────────────────────────────────────────────────────────
          1  C                        │  1  D
          ~                           │  ~
          ~                           │  ~
         c.rs                         │ d.rs
        ────────────────────────────────────────────────────────────
          1  B
          ~
         b.rs
         NORMAL  b.rs                                           1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn move_pane_at_edge_is_noop() {
    // Horizontal { [top, bottom] }. Move top up → no-op
    let buf1 = make_buffer("TOP\n", "top.rs");
    let buf2 = make_buffer("BOT\n", "bot.rs");
    let buf2_id = buf2.id;
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let p2 = state.pane_layout.split(SplitDirection::Horizontal);
    if let Some(pane) = state.pane_layout.pane_by_id_mut(p2) {
        pane.content = PaneContent::Buffer(buf2_id);
    }
    let before = render_to_string(60, 12, &state);
    state.pane_layout.move_direction(FocusDirection::Up);
    let after = render_to_string(60, 12, &state);
    assert_eq!(before, after, "moving edge pane further should be no-op");
}

#[test]
fn move_single_pane_is_noop() {
    let buf1 = make_buffer("ONLY\n", "only.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1);
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(!result, "single pane move should return false");
}

// ─── Tree structure verification tests ───────────────────────────────────
// These test the tree directly (no rendering) to catch structural bugs.

#[test]
fn tree_structure_horizontal_vertical_move_down() {
    // Build: Horizontal { Vertical { [p1, p_v] }, p_h }
    let buf1 = make_buffer("1\n", "1.rs");
    let buf2 = make_buffer("2\n", "2.rs");
    let buf3 = make_buffer("3\n", "3.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p_v = state.pane_layout.split(SplitDirection::Vertical);
    let p1 = state.pane_layout.active_id;

    // Verify initial tree structure
    let tree = state.pane_layout.root.debug_tree();
    assert!(tree.starts_with("H(V("), "expected H(V(...),...)  got {tree}");

    // Move p_v (pane 2, top-right) down
    state.pane_layout.active_id = p_v;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move should succeed");

    // After: should be H(p1, p_h, p_v) — pane 2 at the bottom
    let tree = state.pane_layout.root.debug_tree();
    let ids = state.pane_layout.root.pane_ids();
    assert_eq!(ids, vec![p1, p_h, p_v], "pane order should be [p1, p_h, p_v], tree: {tree}");
}

#[test]
fn tree_structure_horizontal_vertical_move_p1_down() {
    // Build: Horizontal { Vertical { [p1, p_v] }, p_h }
    // Move p1 (top-left) down → H(p_v, p_h, p1)
    let buf1 = make_buffer("1\n", "1.rs");
    let buf2 = make_buffer("2\n", "2.rs");
    let buf3 = make_buffer("3\n", "3.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p_v = state.pane_layout.split(SplitDirection::Vertical);
    let p1 = state.pane_layout.active_id;

    state.pane_layout.active_id = p1;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move should succeed");

    let ids = state.pane_layout.root.pane_ids();
    let tree = state.pane_layout.root.debug_tree();
    assert_eq!(ids, vec![p_v, p_h, p1], "pane order should be [p_v, p_h, p1], tree: {tree}");
}

#[test]
fn tree_structure_move_up_from_bottom() {
    // Build: Horizontal { Vertical { [p1, p_v] }, p_h }
    // Move p_h (bottom) up → H(p_h, V(p1, p_v))
    let buf1 = make_buffer("1\n", "1.rs");
    let buf2 = make_buffer("2\n", "2.rs");
    let buf3 = make_buffer("3\n", "3.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p_v = state.pane_layout.split(SplitDirection::Vertical);
    let p1 = state.pane_layout.active_id;

    state.pane_layout.active_id = p_h;
    let result = state.pane_layout.move_direction(FocusDirection::Up);
    assert!(result, "move should succeed");

    let ids = state.pane_layout.root.pane_ids();
    let tree = state.pane_layout.root.debug_tree();
    assert_eq!(ids, vec![p_h, p1, p_v], "pane order should be [p_h, p1, p_v], tree: {tree}");
}

#[test]
fn tree_structure_same_axis_swap() {
    // Build: Horizontal { Vertical { [p1, p_v] }, p_h }
    // Move p_v right → should stay in Vertical split: H(V(p1, ...), p_h) with p_v moved
    // Actually p_v is already rightmost in V, so moving right crosses axis → wraps at root
    let buf1 = make_buffer("1\n", "1.rs");
    let buf2 = make_buffer("2\n", "2.rs");
    let buf3 = make_buffer("3\n", "3.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p_v = state.pane_layout.split(SplitDirection::Vertical);

    // Move p_v right — same axis (parent Vertical, target Vertical)
    // p_v is at end of Vertical{p1, p_v}, so it wraps root
    state.pane_layout.active_id = p_v;
    let result = state.pane_layout.move_direction(FocusDirection::Right);
    assert!(result, "move should succeed");

    let tree = state.pane_layout.root.debug_tree();
    let ids = state.pane_layout.root.pane_ids();
    // p_v should be at the rightmost position in the Vertical axis
    assert_eq!(*ids.last().unwrap(), p_v, "p_v should be rightmost, tree: {tree}");
}

#[test]
fn tree_structure_same_axis_within_nested() {
    // Build: Vertical { Horizontal{p1, p_h}, p_v }
    // Move p_h down within its column → Vertical { Horizontal{p_h, p1}, p_v }
    let buf1 = make_buffer("1\n", "1.rs");
    let buf2 = make_buffer("2\n", "2.rs");
    let buf3 = make_buffer("3\n", "3.rs");
    let mut state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let p_v = state.pane_layout.split(SplitDirection::Vertical);
    let p_h = state.pane_layout.split(SplitDirection::Horizontal);
    let p1 = state.pane_layout.active_id;

    // Move p1 down within its Horizontal container (same axis: parent H, target H)
    state.pane_layout.active_id = p1;
    let result = state.pane_layout.move_direction(FocusDirection::Down);
    assert!(result, "move should succeed");

    let tree = state.pane_layout.root.debug_tree();
    let ids = state.pane_layout.root.pane_ids();
    // p1 should swap with p_h within the left column
    assert_eq!(ids, vec![p_h, p1, p_v], "tree: {tree}");
}
