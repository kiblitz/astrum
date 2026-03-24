use super::helpers::*;
use expect_test::expect;

#[test]
fn welcome_basic_80x24() {
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
fn welcome_with_config_error() {
    let state = RenderState::default()
        .with_config_error("Config error: invalid key binding 'xyz'");
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

                              Config error: invalid key binding 'xyz'
                                    Using default configuration


         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn welcome_narrow_terminal() {
    let state = RenderState::default();
    let actual = render_to_string(40, 12, &state);
    check(&actual, expect![[r#"
         astrum



             _        _
            / \   ___| |_ _ __ _   _ _ __ ___
           / _ \ / __| __| '__| | | | '_ ` _ \
          / ___ \\__ \ |_| |  | |_| | | | | | |
         /_/   \_\___/\__|_|   \__,_|_| |_| |_|

         NORMAL  Welcome
        SPC for leader | : for commands | SPC q"#]]);
}

#[test]
fn welcome_very_short_terminal() {
    let state = RenderState::default();
    let actual = render_to_string(80, 6, &state);
    check(&actual, expect![[r#"
         astrum



         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn welcome_very_wide_terminal() {
    let state = RenderState::default();
    let actual = render_to_string(160, 24, &state);
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
fn welcome_exactly_logo_width() {
    // Logo is ~40 chars wide; terminal slightly wider
    let state = RenderState::default();
    let actual = render_to_string(50, 24, &state);
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
fn welcome_config_error_long_message() {
    let state = RenderState::default()
        .with_config_error("Config error: unknown key 'leader_key_timeout_ms' in section 'keybindings.normal' at line 42 column 5");
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

          Config error: unknown key 'leader_key_timeout_ms' in section 'keybindings.norm
                                    Using default configuration


         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}

#[test]
fn welcome_minimum_height() {
    // Only 4 rows: tab bar, main (2), status, cmdline — only 2 rows for welcome
    let state = RenderState::default();
    let actual = render_to_string(80, 4, &state);
    check(&actual, expect![[r#"
         astrum

         NORMAL  Welcome
        SPC for leader | : for commands | SPC q q to quit"#]]);
}
