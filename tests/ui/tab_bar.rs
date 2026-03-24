use super::helpers::*;
use astrum::buffer::Buffer;
use expect_test::expect;

#[test]
fn tab_bar_no_buffers() {
    let state = RenderState::default();
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
         astrum



                                 _        _
                                / \   ___| |_ _ __ _   _ _ __ ___
                               / _ \ / __| __| '__| | | | '_ ` _ \
                              / ___ \\__ \ |_| |  | |_| | | | | | |
                             /_/   \_\___/\__|_|   \__,_|_| |_| |_|

                                             v0.1.0


                                       SPC f f  Browse files
                                           :q       Quit

                                 Press SPC for leader key commands





         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn tab_bar_single_buffer() {
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
fn tab_bar_single_modified() {
    let mut buf = make_buffer("hello\n", "main.rs");
    buf.modified = true;
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs [+]
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
         NORMAL  main.rs [+]                                                        1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn tab_bar_multiple_buffers() {
    let buf1 = make_buffer("aaa\n", "main.rs");
    let buf2 = make_buffer("bbb\n", "lib.rs");
    let buf3 = make_buffer("ccc\n", "utils.rs");
    let state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2)
        .with_extra_buffer(buf3);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs  │  lib.rs  │  utils.rs
          1  aaa
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
fn tab_bar_long_filename() {
    let buf = make_buffer("x\n", "this_is_a_very_long_filename_that_might_overflow.rs");
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          this_is_a_very_long_filename_that_might_overflow.rs
          1  x
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
         NORMAL  this_is_a_very_long_filename_that_might_overflow.rs                1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn tab_bar_scratch_buffer() {
    let buf = Buffer::new_scratch();
    let state = RenderState::default().with_buffer(buf);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          [scratch]
          1
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
         NORMAL  [scratch]                                                          1:1
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn tab_bar_modified_and_unmodified_mix() {
    let buf1 = make_buffer("aaa\n", "main.rs");
    let mut buf2 = make_buffer("bbb\n", "lib.rs");
    buf2.modified = true;
    let state = RenderState::default()
        .with_buffer(buf1)
        .with_extra_buffer(buf2);
    let actual = render_to_string(80, 24, &state);
    check(&actual, expect![[r#"
          main.rs  │  lib.rs [+]
          1  aaa
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
