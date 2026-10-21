use super::*;

/// Each page names the panel it wears: menus and pickers the menus' (phase
/// 2), dialogs and the forms a menu turns into theirs (phase 4). Only the
/// status bar's usage panel keeps the old one, until phase 5.
#[test]
fn only_the_usage_panel_keeps_the_old_one() {
    let menus = [
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
    ];
    let forms = [
        Page::CarryPane,
        Page::GitCommit,
        Page::KillProcesses,
        Page::PaneProcesses,
        Page::PrComment,
        Page::PrMerge,
        Page::PrReview,
        Page::RemoveDevice,
        Page::RemoveWsl,
        Page::RenamePane,
        Page::RenameTab,
    ];
    let mut dialogs = vec![
        Page::About,
        Page::AddDevice,
        Page::AddWsl,
        Page::AgentSkill,
        Page::AppUpdate,
        Page::Checkpoints,
        Page::ConfirmClose,
        Page::FanOut,
        Page::ForwardPort,
        Page::GitHub,
        Page::Install,
        Page::OpenLink,
        Page::RenameDevice,
        Page::StartAgent,
        Page::Teleport,
        Page::Update,
        Page::VersionMismatch,
        Page::WorktreeScript,
    ];
    for action in [
        WorkspaceAction::Rename,
        WorkspaceAction::Close,
        WorkspaceAction::NewWorktree,
        WorkspaceAction::OpenWorktree,
        WorkspaceAction::DeleteWorktree,
        WorkspaceAction::NewTab,
        WorkspaceAction::NewWorkspace,
        WorkspaceAction::Note,
    ] {
        dialogs.push(Page::Dialog(action));
    }
    #[cfg(feature = "coder")]
    dialogs.push(Page::AddCoder);
    #[cfg(feature = "cloud")]
    for &provider in crate::cloud::CloudProvider::ALL {
        dialogs.push(Page::AddCloud(provider));
    }
    for page in menus {
        assert_eq!(page.look(), Look::Menu, "{page:?}");
    }
    for page in forms {
        assert_eq!(page.look(), Look::Form, "{page:?}");
    }
    for page in dialogs {
        assert_eq!(page.look(), Look::Dialog, "{page:?}");
    }
    for provider in crate::usage::all_providers() {
        assert_eq!(Page::Usage(provider).look(), Look::Legacy);
    }
}
