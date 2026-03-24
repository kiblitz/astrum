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
