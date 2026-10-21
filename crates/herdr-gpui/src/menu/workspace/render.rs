//! Painting a workspace dialog: its header, the body each action asks for,
//! and the footer whose submit button arms only once there is something to send.

use crate::{
    HerdrWindow,
    design::{ButtonKind, Header, NoteTone},
    menu::{WorkspaceAction, WorkspaceMenuAction, workspace_close::CHECKING},
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// The checkout the daemon would create for the branch currently drafted.
    /// Only a preview: the daemon derives the path it actually uses.
    pub(in crate::menu) fn checkout_preview(&self) -> String {
        let repo = self
            .menu
            .target
            .as_ref()
            .and_then(|target| target.worktree.as_ref())
            .map(|worktree| worktree.label.as_str());
        let root = self
            .live
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.worktree_directory.as_str())
            .filter(|root| !root.is_empty());
        let branch = self
            .menu
            .input
            .as_ref()
            .map_or("", |input| input.text.trim());
        match (repo, root) {
            _ if branch.is_empty() => "The daemon names the checkout.".to_owned(),
            (Some(repo), Some(root)) => crate::worktree::checkout_preview(root, repo, branch),
            _ => "The daemon chooses the checkout.".to_owned(),
        }
    }

    pub(in crate::menu) fn render_workspace_dialog(
        &self,
        action: WorkspaceAction,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let Some(target) = &self.menu.target else {
            return div();
        };
        let font = &self.config.ui;
        let deletion = self.menu.deletion.as_ref();
        let force = deletion.is_some_and(|deletion| deletion.force);
        let removing = action == WorkspaceAction::DeleteWorktree
            && self.removal.as_ref().is_some_and(|removal| {
                removal.pending_for(self.menu.endpoint_target, &target.boot_id, &target.id)
            });
        let dispatching = self.dispatch_job.is_some()
            && matches!(
                action,
                WorkspaceAction::NewWorktree | WorkspaceAction::NewWorkspace
            );
        let creating = self.menu.creation.is_some() || dispatching;
        // Another host chosen in the picker, where the new checkout goes.
        let dispatched = self
            .menu
            .dispatch
            .as_ref()
            .and_then(|picker| picker.dispatched());
        // Until the daemon names the checkout there is nothing to confirm, and a
        // request already in flight leaves nothing to press either.
        let armed = (action != WorkspaceAction::DeleteWorktree
            || deletion.is_some_and(|deletion| deletion.ready()))
            && (action != WorkspaceAction::OpenWorktree
                || self.menu.worktree_open.as_ref().is_some_and(|picker| {
                    picker.pending.is_none()
                        && !picker.filtered.is_empty()
                        && !picker.search.read(cx).is_composing()
                }))
            && (action != WorkspaceAction::Close
                || self.menu.close_check.as_ref().is_some_and(|check| {
                    check.allows(
                        self.menu
                            .input
                            .as_ref()
                            .map_or("", |input| input.text.as_str()),
                        self.config.confirm_close_workspace,
                    )
                }))
            && !creating
            && !removing;
        let destructive = matches!(
            action,
            WorkspaceAction::Close | WorkspaceAction::DeleteWorktree
        );
        let (title, submit): (&str, SharedString) = match action {
            WorkspaceAction::Rename => ("Rename workspace", "Rename".into()),
            WorkspaceAction::Close => (target.close_label(), target.close_label().into()),
            WorkspaceAction::NewWorktree => ("New worktree", create_label(dispatched)),
            WorkspaceAction::OpenWorktree => ("Open worktree", "Open".into()),
            WorkspaceAction::NewTab => ("New tab", "Create".into()),
            WorkspaceAction::NewWorkspace => ("New workspace", create_label(dispatched)),
            WorkspaceAction::Note => ("Note", "Save".into()),
            WorkspaceAction::DeleteWorktree if force => {
                ("Force delete checkout?", "Force remove".into())
            }
            WorkspaceAction::DeleteWorktree => ("Delete worktree checkout?", "Remove".into()),
        };
        let look = self.dialog_look();
        let terminal = &self.config.terminal;
        let mut body = look.body("dialog-body");
        body = match action {
            WorkspaceAction::OpenWorktree => body,
            WorkspaceAction::Rename => body.child(look.lead("dialog-lead", "Edit the workspace label.")),
            WorkspaceAction::Note => body.child(look.lead(
                "dialog-lead",
                "A reminder kept only in this app, on this checkout and branch. Save it empty to remove it.",
            )),
            WorkspaceAction::NewTab => body.child(look.lead(
                "dialog-lead",
                "Name the tab, or leave the suggestion for Herdr to number it.",
            )),
            WorkspaceAction::NewWorkspace => body
                .child(look.lead(
                    "dialog-lead",
                    "Name the workspace, or leave the suggestion for Herdr to name it.",
                ))
                .children(self.menu.dispatch.as_ref().map(|picker| {
                    let caption = |caption: &'static str, field: AnyElement| {
                        look.captions("dialog-host", [(caption, field)])
                    };
                    self.render_host_picker(picker, caption, cx)
                }))
                .children(dispatched.map(|host| {
                    look.note(
                        "dialog-dispatch".into(),
                        NoteTone::Info,
                        format!(
                            "Opens the repository's main checkout on {}{}.",
                            host.label,
                            clone_note(host)
                        ),
                    )
                })),
            WorkspaceAction::Close => body.child(look.lead(
                "dialog-lead",
                format!(
                    "Closes {} workspace(s) and terminates their running terminals. Checkout files and branches are not deleted.",
                    target.close_members.len()
                ),
            )),
            WorkspaceAction::NewWorktree => {
                let mut rows = Vec::new();
                if let Some(source) = self.menu.worktree.as_ref() {
                    let focused = source.name.read(cx).focus.is_focused(window);
                    rows.push((
                        "Name",
                        look.settings()
                            .field("worktree-name".into(), focused)
                            .w_full()
                            .child(div().flex_1().min_w_0().child(source.name.clone()))
                            .into_any_element(),
                    ));
                }
                rows.push(("Branch", self.render_dialog_input(window, cx).into_any_element()));
                body.child(look.captions("dialog-fields", rows))
                    .children(self.menu.dispatch.as_ref().map(|picker| {
                        let caption = |caption: &'static str, field: AnyElement| {
                            look.captions("dialog-host", [(caption, field)])
                        };
                        self.render_host_picker(picker, caption, cx)
                    }))
                    .child(look.lead(
                        "dialog-checkout-lead",
                        match dispatched {
                            Some(host) => format!(
                                "Creates a worktree from {} on {}{}, without granting repository trust:",
                                target.base_label(),
                                host.label,
                                clone_note(host)
                            ),
                            None => format!(
                                "Creates this folder from {}, without granting repository trust:",
                                target.base_label()
                            ),
                        },
                    ))
                    .child(look.well("dialog-checkout".into(), Some(terminal)).child(
                        match dispatched {
                            Some(host) => format!("{}'s daemon chooses the checkout.", host.label),
                            None => self.checkout_preview(),
                        },
                    ))
            }
            WorkspaceAction::DeleteWorktree => body
                .children(deletion.and_then(|deletion| deletion.merged).map(|number| {
                    look.note(
                        "dialog-merged-pr".into(),
                        NoteTone::Done,
                        format!("Pull request #{number} was merged."),
                    )
                }))
                .child(look.lead(
                    "dialog-path-lead",
                    if force {
                        "This force removes the checkout folder:"
                    } else {
                        "This removes the checkout folder:"
                    },
                ))
                .child(
                    look.well("dialog-path".into(), Some(terminal)).child(
                        deletion
                            .and_then(|deletion| deletion.path.as_deref())
                            .unwrap_or("Waiting for the daemon to name the checkout...")
                            .to_owned(),
                    ),
                )
                .child(look.note(
                    "dialog-branch-note".into(),
                    NoteTone::Info,
                    if force {
                        "Modified and untracked files, including submodule contents, are discarded. The branch is not deleted. The Herdr workspace will close."
                    } else {
                        "The branch is not deleted. The Herdr workspace will close."
                    },
                ))
                .children(deletion.and_then(|deletion| {
                    self.archive_note(target, deletion, cx)
                })),
        };
        if action == WorkspaceAction::Close {
            body = body.child(look.note(
                "close-git-status".into(),
                NoteTone::Warning,
                self.menu.close_check.as_ref().map_or_else(
                    || CHECKING.to_owned(),
                    |check| check.status(self.config.confirm_close_workspace),
                ),
            ));
        }
        if self.menu.input.is_some() && action != WorkspaceAction::NewWorktree {
            body = body.child(self.render_dialog_input(window, cx));
        }
        let error = self
            .menu
            .error
            .as_ref()
            .map(|error| look.error("dialog-error".into(), error.clone()));
        // Dismissing only closes the panel; the daemon keeps the queued work.
        let waiting = (creating || removing).then(|| {
            let waiting = match self.dispatch_job.as_ref().filter(|_| dispatching) {
                Some(job) => format!("{} Dismissing does not cancel it.", job.status()),
                None => "Waiting for the daemon. Dismissing does not cancel it.".to_owned(),
            };
            look.note("dialog-waiting".into(), NoteTone::Info, waiting)
        });
        // The new worktree form scrolls inside a fixed height, so the daemon's
        // answer sits under it, above the footer, where no scroll hides it.
        let under_form = action == WorkspaceAction::NewWorktree;
        let (error, waiting) = if under_form {
            (error, waiting)
        } else {
            body = body.children(error).children(waiting);
            (None, None)
        };
        // A picker owns the panel's height, so its list scrolls inside the
        // dialog instead of growing it past the window.
        let listing = action == WorkspaceAction::OpenWorktree || self.worktree_listing();
        let close = look
            .menu()
            .close_button(if action == WorkspaceAction::OpenWorktree {
                "open-worktree-escape"
            } else {
                "dialog-dismiss"
            })
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.dismiss_menu(window, cx);
            }));
        let header = look.header(
            Header {
                selector: "dialog-header",
                icon: WorkspaceMenuAction::Dialog(action)
                    .icon()
                    .unwrap_or("icons/folder.svg"),
                title: ("dialog-title", title),
                sub: Some(("icons/folder.svg", "dialog-target", &target.label)),
                danger: destructive,
            },
            close,
        );
        let submit_label = if creating {
            "Waiting...".into()
        } else {
            submit
        };
        // GitHub rows create their own checkouts without a submit button.
        let submits = !listing || action == WorkspaceAction::OpenWorktree;
        let mut footer = look.footer("dialog-footer").hint("Esc", "cancel");
        if submits {
            footer = footer.hint("↵", submit_word(action));
        }
        footer = footer.button(
            look.button("dialog-cancel", "Cancel", ButtonKind::Normal, true)
                .on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.dismiss_menu(window, cx);
                })),
        );
        if submits {
            footer = footer.button(
                look.button("dialog-submit", submit_label, submit_kind(action), armed)
                    // Inert until there is something to submit, so it never
                    // reads as pressable.
                    .when(!armed, |button| {
                        button.cursor(CursorStyle::OperationNotAllowed)
                    })
                    .when(armed, |button| {
                        button.on_click(cx.listener(|this, _, window, cx| {
                            cx.stop_propagation();
                            this.submit_workspace_dialog(window, cx);
                        }))
                    }),
            );
        }
        div()
            .flex()
            .flex_col()
            .when(
                listing || action == WorkspaceAction::NewWorktree,
                |dialog| dialog.size_full().min_h_0(),
            )
            .child(header)
            .when(action == WorkspaceAction::NewWorktree, |dialog| {
                dialog.child(self.render_worktree_tabs(cx))
            })
            .child(if action == WorkspaceAction::OpenWorktree {
                self.render_existing_worktrees(cx)
                    .when(creating || self.menu.error.is_some(), |picker| {
                        picker.child(
                            div()
                                .id("open-worktree-status")
                                .debug_selector(|| "open-worktree-status".into())
                                .flex_none()
                                .h(px(font.line_height() * 3. + 20.))
                                .overflow_y_scroll()
                                .child(body),
                        )
                    })
                    .into_any_element()
            } else if listing {
                self.render_worktree_items(cx).into_any_element()
            } else if action == WorkspaceAction::NewWorktree {
                // The branch form shares the listings' settled height, so it
                // fills the space above the footer and scrolls within it.
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("new-worktree-form")
                            .debug_selector(|| "new-worktree-form".into())
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(body),
                    )
                    .when(error.is_some() || waiting.is_some(), |column| {
                        column.child(
                            div()
                                .flex_none()
                                .flex()
                                .flex_col()
                                .gap(look.at(8.))
                                .px(look.at(16.))
                                .pb(look.at(12.))
                                .children(error)
                                .children(waiting),
                        )
                    })
                    .into_any_element()
            } else {
                body.into_any_element()
            })
            .child(footer)
    }
}

/// What Enter does, as the footer names it beside its key.
fn submit_word(action: WorkspaceAction) -> &'static str {
    match action {
        WorkspaceAction::Rename => "rename",
        WorkspaceAction::Close => "close",
        WorkspaceAction::NewWorktree | WorkspaceAction::NewTab | WorkspaceAction::NewWorkspace => {
            "create"
        }
        WorkspaceAction::OpenWorktree => "open",
        WorkspaceAction::Note => "save",
        WorkspaceAction::DeleteWorktree => "remove",
    }
}

/// The submit button of a dialog that may create on another host.
fn create_label(dispatched: Option<&crate::dispatch::Candidate>) -> SharedString {
    match dispatched {
        Some(host) => format!("Create on {}", host.label).into(),
        None => "Create".into(),
    }
}

/// What choosing a host without the repository adds to the dialog's text.
fn clone_note(host: &crate::dispatch::Candidate) -> &'static str {
    if host.repository == crate::dispatch::Repository::Missing {
        ", cloning the repository there first"
    } else {
        ""
    }
}

/// The submit button's kind: closing workspaces and removing a checkout
/// destroy something, so they take the danger red.
pub(in crate::menu) fn submit_kind(action: WorkspaceAction) -> ButtonKind {
    match action {
        WorkspaceAction::Close | WorkspaceAction::DeleteWorktree => ButtonKind::Danger,
        WorkspaceAction::Rename
        | WorkspaceAction::NewWorktree
        | WorkspaceAction::OpenWorktree
        | WorkspaceAction::NewTab
        | WorkspaceAction::NewWorkspace
        | WorkspaceAction::Note => ButtonKind::Primary,
    }
}
