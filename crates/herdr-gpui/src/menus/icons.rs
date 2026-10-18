//! The icons the Linux in-window menu leads its entries with. The macOS menu
//! bar is drawn by the system and shows none.

use crate::{
    CheckForUpdates, Quit, RunCommand, ShowLogs,
    actions::{Copy, Cut, Paste, SelectAll},
    constants::WINDOW_TITLE,
    controls::Command,
};

/// The icon of a command the menus offer; `None` for one they do not.
pub(crate) fn command_icon(command: Command) -> Option<&'static str> {
    Some(match command {
        Command::About => "icons/info.svg",
        Command::Back | Command::PreviousTab => "icons/arrow-left.svg",
        Command::ClearPane => "icons/trash.svg",
        Command::CloseTab | Command::ClosePane => "icons/x.svg",
        Command::CloseWindow => "icons/close.svg",
        Command::CopyMode => "icons/copy.svg",
        Command::CycleWindows => "icons/window.svg",
        Command::DecreaseFontSize | Command::IncreaseFontSize | Command::ResetFontSize => {
            "icons/text-size.svg"
        }
        Command::DevicesOverview => "icons/devices.svg",
        Command::EditScrollback | Command::GoToFile => "icons/file.svg",
        Command::Find | Command::WorkspacePicker => "icons/search.svg",
        Command::FindNext => "icons/chevron-down.svg",
        Command::FindPrevious => "icons/chevron-up.svg",
        Command::Forward | Command::NextTab => "icons/arrow-right.svg",
        Command::GoToSymbol => "icons/code.svg",
        Command::Keybinds => "icons/keyboard.svg",
        Command::NewWindow | Command::Workspace => "icons/plus.svg",
        Command::NewWorktree => "icons/git-branch.svg",
        Command::OpenCode => "icons/vscode.svg",
        Command::OpenNotificationTarget => "icons/bell.svg",
        Command::Palette => "icons/command.svg",
        Command::Reconnect => "icons/plug.svg",
        Command::Settings => "icons/settings.svg",
        Command::SplitDown => "icons/split-down.svg",
        Command::SplitRight => "icons/split.svg",
        Command::Tab => "icons/tab.svg",
        Command::ToggleFullScreen => "icons/window-maximize.svg",
        Command::ToggleSidebar => "icons/panel-left.svg",
        Command::ToggleStatusBar => "icons/status-bar.svg",
        Command::ToggleZenMode => "icons/eye.svg",
        Command::Zoom => "icons/zoom.svg",
        _ => return None,
    })
}

/// The icon of a menu entry's action: its command's, or the editing and
/// application actions' own.
pub(crate) fn action_icon(action: &dyn gpui::Action) -> Option<&'static str> {
    let action = action.as_any();
    if let Some(run) = action.downcast_ref::<RunCommand>() {
        return command_icon(run.command);
    }
    if action.is::<Cut>() {
        Some("icons/scissors.svg")
    } else if action.is::<Copy>() {
        Some("icons/copy.svg")
    } else if action.is::<Paste>() {
        Some("icons/clipboard.svg")
    } else if action.is::<SelectAll>() {
        Some("icons/select-all.svg")
    } else if action.is::<CheckForUpdates>() {
        Some("icons/download.svg")
    } else if action.is::<ShowLogs>() {
        Some("icons/file.svg")
    } else if action.is::<Quit>() {
        Some("icons/x.svg")
    } else {
        None
    }
}

/// The icon of a menu or submenu, by the name the menu tree gives it.
pub(crate) fn category_icon(name: &str) -> Option<&'static str> {
    Some(match name {
        WINDOW_TITLE => "icons/info.svg",
        "File" => "icons/file.svg",
        "Edit" => "icons/pencil.svg",
        "View" => "icons/eye.svg",
        "Layout" => "icons/layers.svg",
        "Terminal" => "icons/terminal.svg",
        "Window" => "icons/window.svg",
        _ => return None,
    })
}
