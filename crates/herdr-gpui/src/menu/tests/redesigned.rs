use super::*;

/// Menus and pickers take the redesigned panel; dialogs, forms and
/// Settings' pickers keep theirs until their own phases.
#[test]
fn redesign_dresses_menus_and_pickers_only() {
    for page in [
        Page::Application,
        Page::CodeSearch,
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
        Page::Workspace,
    ] {
        assert!(page.redesigned(), "{page:?}");
    }
    for page in [
        Page::About,
        Page::ConfirmClose,
        Page::DeviceTheme,
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
        Page::Themes,
    ] {
        assert!(!page.redesigned(), "{page:?}");
    }
}
