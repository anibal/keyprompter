use gpui_kit::*;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    library::{Entry, Library},
    platform,
    rundown::{CardId, Edit, Rundown},
    show::Command,
    state::Capture,
    view::StageView,
};

actions!(
    keyprompter,
    [
        Next,
        Prev,
        First,
        Last,
        Import,
        Export,
        Undo,
        Redo,
        DuplicateCard,
        MergeWithNextCard,
        MoveCardEarlier,
        MoveCardLater,
        DeleteCard,
        ToggleCapture,
        MoveToTop,
        BiggerText,
        SmallerText,
        DefaultTextSize,
        Quit,
        Hide,
        HideOthers,
        ShowAll
    ]
);

/// Opens a library rundown by id. Not by menu position: saving reorders the list.
#[derive(Clone, PartialEq, Action)]
#[action(namespace = keyprompter, no_json)]
pub struct OpenRecent {
    pub id: String,
}

/// Moves the strip to the top of the `index`-th screen, as `NSScreen` lists them.
#[derive(Clone, PartialEq, Action)]
#[action(namespace = keyprompter, no_json)]
pub struct MoveToDisplay {
    pub index: usize,
}

/// Key context of the strip. Its bindings live there, so a focused text
/// field's own bindings for the same keys win inside the field.
pub const STRIP: &str = "Strip";

/// The first binding of each action is the one its menu item shows.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("right", Next, Some(STRIP)),
        KeyBinding::new("space", Next, Some(STRIP)),
        KeyBinding::new("pagedown", Next, Some(STRIP)),
        KeyBinding::new("down", Next, Some(STRIP)),
        KeyBinding::new("left", Prev, Some(STRIP)),
        KeyBinding::new("pageup", Prev, Some(STRIP)),
        KeyBinding::new("up", Prev, Some(STRIP)),
        KeyBinding::new("backspace", Prev, Some(STRIP)),
        KeyBinding::new("home", First, Some(STRIP)),
        KeyBinding::new("end", Last, Some(STRIP)),
        KeyBinding::new("cmd-o", Import, None),
        KeyBinding::new("cmd-z", Undo, Some(STRIP)),
        KeyBinding::new("shift-cmd-z", Redo, Some(STRIP)),
        KeyBinding::new("alt-cmd-up", MoveToTop, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
    ]
}

pub fn menus(capture: Capture, screens: &[String], recent: &[Entry]) -> Vec<Menu> {
    vec![
        Menu {
            name: "KeyPrompter".into(),
            items: vec![
                MenuItem::action("Hide KeyPrompter", Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit KeyPrompter", Quit),
            ],
            disabled: false,
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Import Markdown…", Import),
                MenuItem::submenu(Menu {
                    name: "Open Recent".into(),
                    items: recent
                        .iter()
                        .map(|entry| {
                            let title = entry
                                .title
                                .clone()
                                .unwrap_or_else(|| format!("Unreadable rundown {}", entry.id));
                            MenuItem::action(
                                title,
                                OpenRecent {
                                    id: entry.id.clone(),
                                },
                            )
                        })
                        .collect(),
                    disabled: recent.is_empty(),
                }),
                MenuItem::separator(),
                MenuItem::action("Export Markdown…", Export),
            ],
            disabled: false,
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
            ],
            disabled: false,
        },
        Menu {
            name: "Card".into(),
            items: vec![
                MenuItem::action("Duplicate Card", DuplicateCard),
                MenuItem::action("Merge with Next Card", MergeWithNextCard),
                MenuItem::separator(),
                MenuItem::action("Move Card Earlier", MoveCardEarlier),
                MenuItem::action("Move Card Later", MoveCardLater),
                MenuItem::separator(),
                MenuItem::action("Delete Card", DeleteCard),
            ],
            disabled: false,
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Hide from Screen Capture", ToggleCapture)
                    .checked(capture == Capture::Hidden),
                MenuItem::separator(),
                MenuItem::action("Bigger Text", BiggerText),
                MenuItem::action("Smaller Text", SmallerText),
                MenuItem::action("Default Text Size", DefaultTextSize),
                MenuItem::separator(),
                MenuItem::action("Move to Top of Screen", MoveToTop),
                MenuItem::submenu(Menu {
                    name: "Move to Display".into(),
                    items: screens
                        .iter()
                        .enumerate()
                        .map(|(index, name)| {
                            MenuItem::action(name.clone(), MoveToDisplay { index })
                        })
                        .collect(),
                    disabled: screens.is_empty(),
                }),
            ],
            disabled: false,
        },
        Menu {
            name: "Go".into(),
            items: vec![
                MenuItem::action("Next Slide", Next),
                MenuItem::action("Previous Slide", Prev),
                MenuItem::separator(),
                MenuItem::action("First Slide", First),
                MenuItem::action("Last Slide", Last),
            ],
            disabled: false,
        },
    ]
}

/// Binds keys and the actions that need no strip.
pub fn init(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.set_menus(menus(Capture::default(), &[], &[]));
}

/// Routes the strip's actions to its view. They must be global listeners:
/// menu items reach only those, because GPUI's active-window lookup skips
/// panels and the strip is a panel.
pub fn connect(window: AnyWindowHandle, view: Entity<StageView>, library: Library, cx: &mut App) {
    fn route<A: Action>(view: &Entity<StageView>, cmd: Command, cx: &mut App) {
        let view = view.clone();
        cx.on_action(move |_: &A, cx| view.update(cx, |v, cx| v.command(cmd.clone(), cx)));
    }
    route::<Next>(&view, Command::Next, cx);
    route::<Prev>(&view, Command::Prev, cx);
    route::<First>(&view, Command::First, cx);
    route::<Last>(&view, Command::Last, cx);

    fn edit<A: Action>(view: &Entity<StageView>, edit: fn(CardId) -> Edit, cx: &mut App) {
        let view = view.clone();
        cx.on_action(move |_: &A, cx| view.update(cx, |v, cx| v.edit(edit, cx)));
    }
    edit::<DuplicateCard>(&view, Edit::Duplicate, cx);
    edit::<MergeWithNextCard>(&view, Edit::MergeWithNext, cx);
    edit::<MoveCardEarlier>(&view, Edit::MoveEarlier, cx);
    edit::<MoveCardLater>(&view, Edit::MoveLater, cx);
    edit::<DeleteCard>(&view, Edit::Delete, cx);
    cx.on_action({
        let view = view.clone();
        move |_: &Undo, cx| view.update(cx, |v, cx| v.undo(cx))
    });
    cx.on_action({
        let view = view.clone();
        move |_: &Redo, cx| view.update(cx, |v, cx| v.redo(cx))
    });

    cx.on_action({
        let (view, library) = (view.clone(), library.clone());
        move |_: &ToggleCapture, cx| {
            window
                .update(cx, |_, window, cx| {
                    view.update(cx, |v, _| v.toggle_capture(window))
                })
                .ok();
            refresh_menus(&view, &library, cx);
        }
    });
    // GPUI drops the move notification while the window is borrowed here, so
    // settle and save by hand.
    let move_to_top = {
        let view = view.clone();
        move |screen: Option<usize>, cx: &mut App| {
            window
                .update(cx, |_, window, cx| {
                    platform::move_to_top(window, screen);
                    view.update(cx, |v, cx| v.moved(window, cx));
                })
                .ok();
        }
    };
    let text_step = {
        let view = view.clone();
        move |change: fn(i32) -> i32, cx: &mut App| {
            window
                .update(cx, |_, window, cx| {
                    view.update(cx, |v, cx| {
                        v.set_text_step(change(v.text_step()), window, cx)
                    })
                })
                .ok();
        }
    };
    cx.on_action({
        let text_step = text_step.clone();
        move |_: &BiggerText, cx| text_step(|s| s + 1, cx)
    });
    cx.on_action({
        let text_step = text_step.clone();
        move |_: &SmallerText, cx| text_step(|s| s - 1, cx)
    });
    cx.on_action(move |_: &DefaultTextSize, cx| text_step(|_| 0, cx));
    cx.on_action({
        let move_to_top = move_to_top.clone();
        move |_: &MoveToTop, cx| move_to_top(None, cx)
    });
    cx.on_action(move |action: &MoveToDisplay, cx| move_to_top(Some(action.index), cx));
    cx.on_action({
        let (view, library) = (view.clone(), library.clone());
        move |_: &Import, cx| {
            let picked = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Import".into()),
            });
            let (view, library) = (view.clone(), library.clone());
            cx.spawn(async move |cx| {
                if let Ok(Ok(Some(paths))) = picked.await
                    && let Some(path) = paths.into_iter().next()
                {
                    cx.update(|cx| open_markdown(window, &view, &library, &path, cx));
                }
            })
            .detach();
        }
    });
    cx.on_action({
        let view = view.clone();
        move |_: &Export, cx| {
            let Some((_, rundown)) = view.read(cx).rundown().cloned() else {
                view.update(cx, |v, cx| v.show_error("No rundown to export".into(), cx));
                return;
            };
            let dir = rundown
                .imported_from
                .as_deref()
                .and_then(|p| p.parent())
                .map(PathBuf::from)
                .or_else(dirs::home_dir)
                .unwrap_or_default();
            let picked = cx.prompt_for_new_path(&dir, Some(&format!("{}.md", rundown.title)));
            let view = view.clone();
            cx.spawn(async move |cx| {
                if let Ok(Ok(Some(path))) = picked.await
                    && let Err(e) = std::fs::write(&path, rundown.export())
                {
                    cx.update(|cx| {
                        view.update(cx, |v, cx| v.show_error(format!("Can't export: {e}"), cx))
                    });
                }
            })
            .detach();
        }
    });
    cx.on_action({
        let (view, library) = (view.clone(), library.clone());
        move |action: &OpenRecent, cx| {
            if view
                .read(cx)
                .rundown()
                .is_some_and(|(id, _)| *id == action.id)
            {
                return;
            }
            match library.load(&action.id) {
                Ok(rundown) => open(window, &view, action.id.clone(), rundown, cx),
                Err(e) => view.update(cx, |v, cx| {
                    v.show_error(format!("Can't open rundown {}: {e}", action.id), cx)
                }),
            }
            refresh_menus(&view, &library, cx);
        }
    });
    refresh_menus(&view, &library, cx);
    cx.spawn(async move |cx| {
        let mut shown = platform::screen_names();
        loop {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let now = platform::screen_names();
            if now != shown {
                shown = now;
                cx.update(|cx| refresh_menus(&view, &library, cx));
            }
        }
    })
    .detach();
}

/// Rebuilds the menu bar from the capture setting, the connected screens
/// and the library.
fn refresh_menus(view: &Entity<StageView>, library: &Library, cx: &mut App) {
    cx.set_menus(menus(
        view.read(cx).capture(),
        &platform::screen_names(),
        &library.list(),
    ));
}

pub fn open(
    window: AnyWindowHandle,
    view: &Entity<StageView>,
    id: String,
    rundown: Rundown,
    cx: &mut App,
) {
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |v, cx| v.open(id, rundown, window, cx))
        })
        .ok();
}

/// Opens a Markdown file through the library, importing it the first time.
pub fn open_markdown(
    window: AnyWindowHandle,
    view: &Entity<StageView>,
    library: &Library,
    path: &Path,
    cx: &mut App,
) {
    match library.open_markdown(path) {
        Ok((id, rundown)) => open(window, view, id, rundown, cx),
        Err(e) => view.update(cx, |v, cx| {
            v.show_error(format!("Can't import {}: {e}", path.display()), cx)
        }),
    }
    refresh_menus(view, library, cx);
}
