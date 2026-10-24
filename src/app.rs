use gpui_kit::*;

actions!(
    keyprompter,
    [
        Next, Prev, First, Last, Open, Quit, Hide, HideOthers, ShowAll
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

pub fn menus() -> Vec<Menu> {
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

/// Binds keys and app-wide actions. `Open` is left to the caller, which owns the deck.
pub fn init(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.set_menus(menus());
}
