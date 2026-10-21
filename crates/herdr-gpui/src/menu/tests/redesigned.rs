use super::*;

/// Each page names the panel it wears. Menus and pickers took the
/// redesigned one in phase 2; phase 4 moves dialogs and forms page by page.
#[test]
fn every_page_has_a_look() {
    for page in [
        Page::Application,
        Page::CodeSearch,
        Page::DeviceTheme,
        Page::Devices,
        Page::Git,
        Page::Group,
        Page::Host,
        Page::Keybinds,
        Page::Menu,
        Page::MovePane,
        Page::Palette,
        Page::Pane,
        Page::Sessions,
        Page::Tab,
        Page::Themes,
        Page::Workspace,
    ] {
        assert_eq!(page.look(), Look::Menu, "{page:?}");
    }
    assert_eq!(Page::Install.look(), Look::Dialog);
    assert_eq!(Page::RenameTab.look(), Look::Form);
    assert_eq!(Page::AddDevice.look(), Look::Legacy);
}
