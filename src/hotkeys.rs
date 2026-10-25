use futures::{StreamExt, channel::mpsc};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use gpui_kit::*;

use crate::{show::Command, view::StageView};

/// System-wide shortcuts, so slides advance while OBS or a browser is frontmost.
/// Carbon hotkeys need no Accessibility permission.
pub fn listen(window: AnyWindowHandle, view: Entity<StageView>, cx: &mut App) {
    let bindings = [
        (Code::ArrowRight, Command::Next),
        (Code::ArrowLeft, Command::Prev),
    ]
    .map(|(code, cmd)| {
        (
            HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), code),
            cmd,
        )
    });
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(e) => return eprintln!("global hotkeys unavailable: {e}"),
    };
    for (hotkey, _) in &bindings {
        if let Err(e) = manager.register(*hotkey) {
            eprintln!("can't register {hotkey}: {e}");
        }
    }
    let (pressed_tx, mut pressed_rx) = mpsc::unbounded();
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        if e.state == HotKeyState::Pressed {
            pressed_tx.unbounded_send(e.id).ok();
        }
    }));
    cx.spawn(async move |cx| {
        let _registered = manager;
        while let Some(id) = pressed_rx.next().await {
            if let Some((_, cmd)) = bindings.iter().find(|(hotkey, _)| hotkey.id() == id) {
                window
                    .update(cx, |_, window, cx| {
                        view.update(cx, |v, cx| v.command(*cmd, window, cx))
                    })
                    .ok();
            }
        }
    })
    .detach();
}
