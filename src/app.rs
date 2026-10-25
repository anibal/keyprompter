use gpui_kit::*;

use std::{path::PathBuf, time::Duration};

use crate::{platform, show::Command, state::Capture, view::StageView};

actions!(
    keyprompter,
    [
        Next,
        Prev,
        First,
        Last,
        Open,
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

/// Moves the strip to the top of the `index`-th screen, as `NSScreen` lists them.
#[derive(Clone, PartialEq, Action)]
#[action(namespace = keyprompter, no_json)]
pub struct MoveToDisplay {
    pub index: usize,
}

/// The first binding of each action is the one its menu item shows.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("right", Next, None),
        KeyBinding::new("space", Next, None),
        KeyBinding::new("pagedown", Next, None),
        KeyBinding::new("down", Next, None),
        KeyBinding::new("left", Prev, None),
        KeyBinding::new("pageup", Prev, None),
        KeyBinding::new("up", Prev, None),
        KeyBinding::new("backspace", Prev, None),
        KeyBinding::new("home", First, None),
        KeyBinding::new("end", Last, None),
        KeyBinding::new("cmd-o", Open, None),
        KeyBinding::new("alt-cmd-up", MoveToTop, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
    ]
}

pub fn menus(capture: Capture, screens: &[String]) -> Vec<Menu> {
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
            items: vec![MenuItem::action("Open…", Open)],
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
    cx.set_menus(menus(Capture::default(), &[]));
}

/// Routes the strip's actions to its view. They must be global listeners:
/// menu items reach only those, because GPUI's active-window lookup skips
/// panels and the strip is a panel.
pub fn connect(window: AnyWindowHandle, view: Entity<StageView>, cx: &mut App) {
    fn route<A: Action>(view: &Entity<StageView>, cmd: Command, cx: &mut App) {
        let view = view.clone();
        cx.on_action(move |_: &A, cx| view.update(cx, |v, cx| v.command(cmd.clone(), cx)));
    }
    route::<Next>(&view, Command::Next, cx);
    route::<Prev>(&view, Command::Prev, cx);
    route::<First>(&view, Command::First, cx);
    route::<Last>(&view, Command::Last, cx);

    cx.on_action({
        let view = view.clone();
        move |_: &ToggleCapture, cx| {
            window
                .update(cx, |_, window, cx| {
                    view.update(cx, |v, _| v.toggle_capture(window))
                })
                .ok();
            refresh_menus(&view, cx);
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
        let view = view.clone();
        move |_: &Open, cx| {
            let picked = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Open Rundown".into()),
            });
            let view = view.clone();
            cx.spawn(async move |cx| {
                if let Ok(Ok(Some(paths))) = picked.await
                    && let Some(path) = paths.into_iter().next()
                {
                    cx.update(|cx| open(window, &view, path, cx));
                }
            })
            .detach();
        }
    });
    refresh_menus(&view, cx);
    cx.spawn(async move |cx| {
        let mut shown = platform::screen_names();
        loop {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            let now = platform::screen_names();
            if now != shown {
                shown = now;
                cx.update(|cx| refresh_menus(&view, cx));
            }
        }
    })
    .detach();
}

/// Rebuilds the menu bar from the capture setting and the connected screens.
fn refresh_menus(view: &Entity<StageView>, cx: &mut App) {
    cx.set_menus(menus(view.read(cx).capture(), &platform::screen_names()));
}

pub fn open(window: AnyWindowHandle, view: &Entity<StageView>, path: PathBuf, cx: &mut App) {
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |v, cx| v.open(path, window, cx))
        })
        .ok();
}
