use std::path::PathBuf;

use futures::{StreamExt, channel::mpsc};
use gpui_kit::*;
use keyprompter::{
    app::{self, Open},
    state::{self, Frame},
    view::StageView,
};

const DEFAULT_H: f32 = 150.0;

fn main() {
    let state_path = state::path();
    let saved = state_path.as_deref().map(state::load).unwrap_or_default();
    let deck = std::env::args_os().nth(1).map(PathBuf::from).or(saved.deck);

    let (opened_tx, mut opened_rx) = mpsc::unbounded();
    let application = application();
    application.on_open_urls(move |urls| {
        for path in urls
            .iter()
            .filter_map(|u| url::Url::parse(u).ok()?.to_file_path().ok())
        {
            opened_tx.unbounded_send(path).ok();
        }
    });

    application.run(move |cx| {
        init(cx);
        app::init(cx);
        let frame = saved.frame.unwrap_or_else(|| Frame {
            x: 0.0,
            y: 0.0,
            w: cx
                .primary_display()
                .map_or(1440.0, |d| f32::from(d.bounds().size.width)),
            h: DEFAULT_H,
        });
        let (_, view) = open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(frame.x), px(frame.y)),
                    size: size(px(frame.w), px(frame.h)),
                })),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: true,
                window_background: WindowBackgroundAppearance::Blurred,
                ..WindowOptions::default()
            },
            cx,
            |window, cx| cx.new(|cx| StageView::new(deck, state_path, window, cx)),
        )
        .expect("open window");

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
                        view.update(cx, |v, cx| v.open(path, cx));
                    }
                })
                .detach();
            }
        });
        cx.spawn(async move |cx| {
            while let Some(path) = opened_rx.next().await {
                view.update(cx, |v, cx| v.open(path, cx));
            }
        })
        .detach();
        cx.activate(true);
    });
}
