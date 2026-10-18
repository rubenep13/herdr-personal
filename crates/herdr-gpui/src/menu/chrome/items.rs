//! The footer menu's entries: the id its handlers match on, the label it
//! shows, and its icon, each declared once.

/// `(id, label, icon)` for every entry `menu_items` can list.
pub(in crate::menu) const ITEMS: &[(&str, &str, &str)] = &[
    ("settings", "Settings", "icons/settings.svg"),
    ("shortcuts", "Shortcuts", "icons/keyboard.svg"),
    ("themes", "Themes", "icons/theme.svg"),
    (
        "increase font size",
        "Increase Font Size",
        "icons/text-size.svg",
    ),
    (
        "decrease font size",
        "Decrease Font Size",
        "icons/text-size.svg",
    ),
    ("reset font size", "Reset Font Size", "icons/text-size.svg"),
    ("command palette", "Command Palette", "icons/command.svg"),
    ("workspaces", "Workspaces", "icons/layers.svg"),
    (
        "reload GUI config",
        "Reload GUI Config",
        "icons/refresh.svg",
    ),
    ("app updates", "App Updates", "icons/download.svg"),
    (
        "preview app update",
        "Preview App Update",
        "icons/download.svg",
    ),
    ("GitHub sign-in", "GitHub Sign-in", "icons/github.svg"),
    ("about", "About", "icons/info.svg"),
    (
        "reload daemon config",
        "Reload Daemon Config",
        "icons/refresh.svg",
    ),
    ("update ready", "Update Ready", "icons/download.svg"),
    ("what's new", "What's New", "icons/info.svg"),
    ("detach", "Detach", "icons/plug.svg"),
    ("reconnect", "Reconnect", "icons/plug.svg"),
];

/// The label `id` shows, or the id itself if it has none.
pub(in crate::menu) fn label(id: &str) -> &str {
    ITEMS
        .iter()
        .find(|(item, _, _)| *item == id)
        .map_or(id, |(_, label, _)| label)
}

pub(in crate::menu) fn icon(id: &str) -> Option<&'static str> {
    ITEMS
        .iter()
        .find(|(item, _, _)| *item == id)
        .map(|(_, _, icon)| *icon)
}
