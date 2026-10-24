use gpui_kit::*;

use std::path::PathBuf;

use crate::{show::Command, state::Capture, view::StageView};

actions!(
    keyprompter,
    [
        Next,
        Prev,
        First,
        Last,
        Open,
        ToggleCapture,
        Quit,
        Hide,
        HideOthers,
        ShowAll
    ]
);

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
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
    ]
}

pub fn menus(capture: Capture) -> Vec<Menu> {
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
    cx.set_menus(menus(Capture::default()));
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
            cx.set_menus(menus(view.read(cx).capture()));
        }
    });
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
    cx.set_menus(menus(view.read(cx).capture()));
}

pub fn open(window: AnyWindowHandle, view: &Entity<StageView>, path: PathBuf, cx: &mut App) {
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |v, cx| v.open(path, window, cx))
        })
        .ok();
}
