use super::*;

/// Menus and pickers take the redesigned panel; dialogs and forms keep
/// theirs until their own phase.
#[test]
fn redesign_dresses_menus_and_pickers_only() {
    for page in [
        Page::Application,
        Page::CodeSearch,
        Page::DeviceTheme,
        Page::Devices,
        Page::Git,
        Page::Group,
        Page::Host,
        Page::Menu,
        Page::MovePane,
        Page::Palette,
        Page::Pane,
        Page::Sessions,
        Page::Tab,
        Page::Themes,
        Page::Workspace,
    ] {
        assert!(page.redesigned(), "{page:?}");
    }
    for page in [
        Page::About,
        Page::ConfirmClose,
        Page::Dialog(WorkspaceAction::Rename),
        Page::ForwardPort,
        Page::GitCommit,
        Page::Keybinds,
        Page::KillProcesses,
        Page::PaneProcesses,
        Page::RemoveDevice,
        Page::RenameDevice,
        Page::RenamePane,
        Page::RenameTab,
    ] {
        assert!(!page.redesigned(), "{page:?}");
    }
}
