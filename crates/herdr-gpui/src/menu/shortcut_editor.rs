//! Editing GUI overrides from the shortcut reference. The field uses GPUI's
//! key notation, so aliases are editable without capturing desktop shortcuts.
use super::Page;
use crate::design::{ButtonKind, ButtonSize};
use crate::{
    HerdrWindow,
    config::Config,
    controls::CommandInfo,
    keymap::{Binding, DaemonKeys, Keymap},
    search_input::SearchInput,
};
use gpui::{Context, Div, Entity, FontWeight, KeyDownEvent, Window, div, prelude::*, px};

pub(super) struct Editor {
    info: &'static CommandInfo,
    input: Entity<SearchInput>,
    saving: bool,
    error: Option<String>,
}

#[derive(Clone, Copy)]
enum Edit {
    Save,
    Unbind,
    Reset,
}

impl HerdrWindow {
    pub(super) fn edit_shortcut(
        &mut self,
        info: &'static CommandInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .menu
            .shortcut_editor
            .as_ref()
            .is_some_and(|editor| editor.saving)
        {
            return;
        }
        let keys = self
            .keymap()
            .bindings()
            .filter(|(command, _)| *command == info.command)
            .map(|(_, key)| key)
            .collect::<Vec<_>>()
            .join(" ");
        let input = cx.new(SearchInput::new);
        // The editor draws the field's frame.
        input.update(cx, |input, cx| input.set_frameless(cx));
        input.update(cx, |input, cx| {
            input.set_placeholder("e.g. ctrl-shift-t", cx);
            input.set_appearance(self.config.ui.clone(), self.theme.clone(), cx);
            input.set_text_selected(&keys, cx);
            window.focus(&input.focus, cx);
        });
        self.menu.shortcut_editor = Some(Editor {
            info,
            input,
            saving: false,
            error: None,
        });
        self.menu.keybinds_scroll.set_offset(Default::default());
        cx.notify();
    }

    fn cancel_shortcut_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu.shortcut_editor = None;
        if let Some(search) = &self.menu.keybinds_search {
            window.focus(&search.read(cx).focus.clone(), cx);
        }
        cx.notify();
    }

    pub(super) fn shortcut_editor_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(editor) = self
            .menu
            .shortcut_editor
            .as_ref()
            .filter(|_| self.menu.page == Some(Page::Keybinds))
        else {
            return false;
        };
        let input = editor.input.read(cx);
        if !input.focus.is_focused(window) {
            return false;
        }
        if input.is_composing() {
            return true;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.cancel_shortcut_edit(window, cx),
            "enter" => self.save_shortcut_edit(Edit::Save, window, cx),
            _ => return true, // The field owns typing and native text editing.
        }
        cx.stop_propagation();
        window.prevent_default();
        true
    }

    fn save_shortcut_edit(&mut self, edit: Edit, window: &mut Window, cx: &mut Context<Self>) {
        self.save_shortcut_with(edit, Config::save_keybinding, window, cx);
    }

    fn save_shortcut_with(
        &mut self,
        edit: Edit,
        save: impl FnOnce(&'static str, Option<Binding>) -> crate::Result<()> + Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self
            .menu
            .shortcut_editor
            .as_mut()
            .filter(|editor| !editor.saving)
        else {
            return;
        };
        if editor.input.read(cx).is_composing() {
            return;
        }
        let binding = match edit {
            Edit::Reset => None,
            Edit::Unbind => Some(Binding::Many(Vec::new())),
            Edit::Save => Some(Binding::Many(
                editor
                    .input
                    .read(cx)
                    .text()
                    .split_whitespace()
                    .take(9) // The keymap rejects more than eight aliases.
                    .map(str::to_owned)
                    .collect(),
            )),
        };
        let name = editor.info.name;
        let mut overrides = self.config.keybinding_overrides.clone();
        if let Some(binding) = &binding {
            overrides.insert(name.into(), binding.clone());
        } else {
            overrides.remove(name);
        }
        if let Err(error) =
            Keymap::with_overrides(&overrides, &self.config.pane_keys, &DaemonKeys::default())
        {
            editor.error = Some(error.to_string());
            cx.notify();
            return;
        }
        editor.error = None;
        editor.saving = true;
        let input = editor.input.clone();
        let saved = cx
            .background_executor()
            .spawn(async move { save(name, binding) });
        cx.spawn_in(window, async move |this, cx| {
            let result = saved.await;
            let _ = this.update_in(cx, |this, window, cx| {
                let current = this
                    .menu
                    .shortcut_editor
                    .as_ref()
                    .is_some_and(|editor| editor.input == input);
                match result {
                    Ok(()) => {
                        if current {
                            this.cancel_shortcut_edit(window, cx);
                        }
                        this.load_gui_config(cx);
                    }
                    Err(error) => {
                        if current && let Some(editor) = &mut this.menu.shortcut_editor {
                            editor.saving = false;
                            editor.error = Some(error.to_string());
                        } else {
                            this.local_error = Some(format!("Save keyboard shortcut: {error}"));
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn render_shortcut_editor(&self, cx: &mut Context<Self>) -> Option<Div> {
        let editor = self.menu.shortcut_editor.as_ref()?;
        let look = crate::design::SettingsLook::of(&self.theme, &self.config.ui);
        let enabled = !editor.saving;
        let buttons = [
            ("shortcut-save", "Save", Edit::Save, ButtonKind::Primary),
            (
                "shortcut-unbind",
                "Unbind",
                Edit::Unbind,
                ButtonKind::Normal,
            ),
            ("shortcut-reset", "Reset", Edit::Reset, ButtonKind::Normal),
        ]
        .into_iter()
        .map(|(id, label, edit, kind)| {
            look.button(id, id.into(), label, kind, ButtonSize::Small, enabled)
                .when(enabled, |button| {
                    button.on_click(cx.listener(move |this, _, window, cx| {
                        this.save_shortcut_edit(edit, window, cx);
                    }))
                })
        });
        let note = |text: &'static str| {
            div()
                .text_size(px(look.scale().secondary))
                .text_color(look.muted())
                .child(text)
        };
        let panel = div()
            .debug_selector(|| "shortcut-editor".into())
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.at(6.))
            .mx(look.at(10.))
            .mb(look.at(6.))
            .p(look.at(12.))
            .rounded(look.at(10.))
            .bg(look.card_fill())
            .border_1()
            .border_color(look.line())
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(look.title())
                    .child(format!("Edit {}", editor.info.label)),
            )
            .child(
                look.field("shortcut-editor-field".into(), false)
                    .child(div().flex_1().min_w_0().child(editor.input.clone())),
            )
            .child(note(
                "Type key names with modifiers, e.g. ctrl-shift-t. Separate alternatives with spaces.",
            ))
            .child(note(
                "Save replaces this command's shortcuts, including daemon chords. Reset restores inherited defaults.",
            ))
            .when_some(editor.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .debug_selector(|| "shortcut-error".into())
                        .text_color(look.danger())
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.at(8.))
                    .mt(look.at(4.))
                    .children(buttons)
                    .child(
                        look.button(
                            "shortcut-cancel",
                            "shortcut-cancel".into(),
                            "Cancel",
                            ButtonKind::Normal,
                            ButtonSize::Small,
                            true,
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.cancel_shortcut_edit(window, cx)),
                        ),
                    )
                    .when(editor.saving, |row| {
                        row.child(div().text_color(look.muted()).child("Saving…"))
                    }),
            );
        Some(panel)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests;

#[cfg(all(feature = "integration-test", target_os = "linux"))]
mod native;
#[cfg(all(feature = "integration-test", target_os = "linux"))]
pub(crate) use native::verify as verify_native;
