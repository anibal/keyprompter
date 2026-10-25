use std::path::PathBuf;

use futures::{StreamExt, channel::mpsc};
use gpui_kit::*;
use keyprompter::{
    app, hotkeys,
    library::Library,
    state::{self, Frame},
    view::StageView,
};

const DEFAULT_H: f32 = 150.0;

fn main() {
    let state_path = state::path();
    let saved = state_path.as_deref().map(state::load).unwrap_or_default();
    let library = Library::new(Library::default_dir().expect("Application Support folder"));
    let argument = std::env::args_os()
        .nth(1)
        .map(|arg| std::path::absolute(&arg).unwrap_or_else(|_| PathBuf::from(arg)));
    let (restored, restore_error) = match (&argument, &saved.rundown) {
        (None, Some(id)) => match library.load(id) {
            Ok(rundown) => (Some((id.clone(), rundown)), None),
            Err(e) => (None, Some(format!("Can't reopen the last rundown: {e}"))),
        },
        _ => (None, None),
    };

    let (opened_tx, mut opened_rx) = mpsc::unbounded();
    if let Some(path) = argument {
        opened_tx.unbounded_send(path).ok();
    }
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
        let (window, view) = open_window(
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
            |window, cx| cx.new(|cx| StageView::new(restored, saved, state_path, window, cx)),
        )
        .expect("open window");

        app::connect(window, view.clone(), library.clone(), cx);
        if let Some(error) = restore_error {
            view.update(cx, |v, cx| v.show_error(error, cx));
        }
        hotkeys::listen(view.clone(), cx);
        cx.spawn(async move |cx| {
            while let Some(path) = opened_rx.next().await {
                cx.update(|cx| app::open_markdown(window, &view, &library, &path, cx));
            }
        })
        .detach();
        cx.activate(true);
    });
}
