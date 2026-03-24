use super::helpers::*;
use astrum::action::Action;
use expect_test::expect;

#[test]
fn palette_all_items() {
    let palette = make_palette(vec![
        ("Save buffer", "SPC f s", Action::SaveBuffer),
        ("Open file browser", "SPC f f", Action::OpenFileBrowser),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Save buffer                            SPC f s│
          ~             │Open file browser                      SPC f f│
          ~             │Quit                                   SPC q q│
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
fn palette_with_query() {
    let mut palette = make_palette(vec![
        ("Save buffer", "SPC f s", Action::SaveBuffer),
        ("Open file browser", "SPC f f", Action::OpenFileBrowser),
        ("Split vertical", "SPC w v", Action::SplitVertical),
    ]);
    palette.query = "save".to_string();
    palette.filtered = vec![0]; // Only "Save buffer" matches.
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │> save                                        │
          ~             │Save buffer                            SPC f s│
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
fn palette_selected_second() {
    let mut palette = make_palette(vec![
        ("Save buffer", "SPC f s", Action::SaveBuffer),
        ("Open file browser", "SPC f f", Action::OpenFileBrowser),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    palette.selected = 1;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Save buffer                            SPC f s│
          ~             │Open file browser                      SPC f f│
          ~             │Quit                                   SPC q q│
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
fn palette_many_items() {
    let items: Vec<_> = (0..15)
        .map(|i| (
            Box::leak(format!("Action {}", i).into_boxed_str()) as &str,
            Box::leak(format!("SPC {}", i).into_boxed_str()) as &str,
            Action::Noop,
        ))
        .collect();
    let palette = make_palette(items);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Action 0                                 SPC 0│
          ~             │Action 1                                 SPC 1│
          ~             │Action 2                                 SPC 2│
          ~             │Action 3                                 SPC 3│
          ~             │Action 4                                 SPC 4│
          ~             │Action 5                                 SPC 5│
          ~             │Action 6                                 SPC 6│
          ~             │Action 7                                 SPC 7│
          ~             │Action 8                                 SPC 8│
          ~             │Action 9                                 SPC 9│
          ~             │Action 10                               SPC 10│
          ~             │Action 11                               SPC 11│
          ~             │Action 12                               SPC 12│
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn palette_small_terminal() {
    let palette = make_palette(vec![
        ("Save", "SPC f s", Action::SaveBuffer),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(40, 12, &state);
    check(&actual, expect![[r#"
          main.rs
          ┌ Actions (type to filter) ────────┐
          │>                                 │
          │Save                       SPC f s│
          │Quit                       SPC q q│
          │                                  │
          │                                  │
          │                                  │
          │                                  │
          │                                  │
         N└──────────────────────────────────┘1
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn palette_single_item() {
    let palette = make_palette(vec![
        ("Quit", "SPC q q", Action::Quit),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Quit                                   SPC q q│
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
fn palette_selected_last() {
    let mut palette = make_palette(vec![
        ("Save", "SPC f s", Action::SaveBuffer),
        ("Open", "SPC f f", Action::OpenFileBrowser),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    palette.selected = 2;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Save                                   SPC f s│
          ~             │Open                                   SPC f f│
          ~             │Quit                                   SPC q q│
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
fn palette_long_action_name() {
    let palette = make_palette(vec![
        ("Toggle line comment for current selection", "SPC ; ;", Action::Noop),
        ("Save", "SPC f s", Action::SaveBuffer),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Toggle line comment for current selection  SPC│
          ~             │Save                                   SPC f s│
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
fn palette_scrolled_to_bottom() {
    let items: Vec<_> = (0..30)
        .map(|i| (
            Box::leak(format!("Action {}", i).into_boxed_str()) as &str,
            Box::leak(format!("SPC {}", i).into_boxed_str()) as &str,
            Action::Noop,
        ))
        .collect();
    let mut palette = make_palette(items);
    palette.selected = 25;
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │>                                             │
          ~             │Action 13                               SPC 13│
          ~             │Action 14                               SPC 14│
          ~             │Action 15                               SPC 15│
          ~             │Action 16                               SPC 16│
          ~             │Action 17                               SPC 17│
          ~             │Action 18                               SPC 18│
          ~             │Action 19                               SPC 19│
          ~             │Action 20                               SPC 20│
          ~             │Action 21                               SPC 21│
          ~             │Action 22                               SPC 22│
          ~             │Action 23                               SPC 23│
          ~             │Action 24                               SPC 24│
          ~             │Action 25                               SPC 25│
          ~             └──────────────────────────────────────────────┘
          ~
          ~
         NORMAL  main.rs                                                            1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn palette_very_wide_terminal() {
    let palette = make_palette(vec![
        ("Save", "SPC f s", Action::SaveBuffer),
        ("Quit", "SPC q q", Action::Quit),
    ]);
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(160, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~                             ┌ Actions (type to filter) ────────────────────────────────────────────────────────────────────┐
          ~                             │>                                                                                             │
          ~                             │Save                                                                                   SPC f s│
          ~                             │Quit                                                                                   SPC q q│
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
fn palette_with_long_query() {
    let mut palette = make_palette(vec![
        ("Save buffer to disk", "SPC f s", Action::SaveBuffer),
        ("Quit application", "SPC q q", Action::Quit),
    ]);
    palette.query = "save buffer".to_string();
    palette.filtered = vec![0];
    let buf = make_buffer("hello\n", "main.rs");
    let state = RenderState::default()
        .with_buffer(buf)
        .with_palette(palette);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs
          1  hello
          ~
          ~
          ~             ┌ Actions (type to filter) ────────────────────┐
          ~             │> save buffer                                 │
          ~             │Save buffer to disk                    SPC f s│
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
