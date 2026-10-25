use std::{
    fs::File,
    io::{BufRead, BufReader},
    os::unix::fs::FileTypeExt,
    path::PathBuf,
};

use futures::{StreamExt, channel::mpsc};
use gpui_kit::*;
use keyprompter::{
    app, document::Document, hotkeys, library::Library, snap, state, view::StageView,
};

/// Points KeyPrompter at another settings and library folder. A test run there
/// also reads actions line by line from its `drive` pipe, each a name with
/// any data as JSON after a space, such as `Next` or `EditCard "Delete"`.
const TEST_HOME: &str = "KEYPROMPTER_HOME";

enum Request {
    Import(PathBuf),
    Action(String),
}

fn main() {
    let test_home = std::env::var_os(TEST_HOME).map(PathBuf::from);
    if let Some(home) = &test_home {
        std::fs::write(home.join("pid"), std::process::id().to_string()).ok();
    }
    let drive = test_home.as_ref().map(|home| home.join("drive"));
    let home = test_home
        .or_else(|| Some(dirs::config_dir()?.join("KeyPrompter")))
        .expect("Application Support folder");
    let state_path = Some(home.join("state.txt"));
    let saved = state_path.as_deref().map(state::load).unwrap_or_default();
    let library = Library::new(home.join("Rundowns"));
    let argument = std::env::args_os()
        .nth(1)
        .map(|arg| std::path::absolute(&arg).unwrap_or_else(|_| PathBuf::from(arg)));
    let (restored, restore_error) = match (&argument, &saved.rundown) {
        (None, Some(id)) => match library.load(id) {
            Ok(rundown) => (Some(Document::new(id.clone(), rundown)), None),
            Err(e) => (None, Some(format!("Can't reopen the last rundown: {e}"))),
        },
        _ => (None, None),
    };

    let (requests_tx, mut requests_rx) = mpsc::unbounded();
    if let Some(path) = argument {
        requests_tx.unbounded_send(Request::Import(path)).ok();
    }
    let application = application();
    if let Some(pipe) = drive.filter(|p| p.metadata().is_ok_and(|m| m.file_type().is_fifo())) {
        let requests_tx = requests_tx.clone();
        std::thread::spawn(move || {
            while let Ok(file) = File::open(&pipe) {
                for name in BufReader::new(file).lines().map_while(Result::ok) {
                    requests_tx.unbounded_send(Request::Action(name)).ok();
                }
            }
        });
    }
    application.on_open_urls(move |urls| {
        for path in urls
            .iter()
            .filter_map(|u| url::Url::parse(u).ok()?.to_file_path().ok())
        {
            requests_tx.unbounded_send(Request::Import(path)).ok();
        }
    });

    application.run(move |cx| {
        init(cx);
        app::init(cx);
        // The saved placement goes on once the window is on screen; see
        // StageView::new.
        let w = cx
            .primary_display()
            .map_or(1440.0, |d| f32::from(d.bounds().size.width));
        let h = saved.placement.as_ref().map_or(snap::DEFAULT_H, |p| p.h);
        let (window, view) = open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(0.0), px(0.0)),
                    size: size(px(w), px(h)),
                })),
                titlebar: None,
                kind: WindowKind::PopUp,
                is_movable: true,
                window_background: WindowBackgroundAppearance::Blurred,
                ..WindowOptions::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    StageView::new(restored, library.clone(), saved, state_path, window, cx)
                })
            },
        )
        .expect("open window");

        app::connect(window, view.clone(), library.clone(), cx);
        if let Some(error) = restore_error {
            view.update(cx, |v, cx| v.show_error(error, cx));
        }
        hotkeys::listen(window, view.clone(), cx);
        cx.spawn(async move |cx| {
            while let Some(request) = requests_rx.next().await {
                cx.update(|cx| match request {
                    Request::Import(path) => app::open_markdown(window, &view, &library, &path, cx),
                    Request::Action(line) => {
                        let (name, data) = line.split_once(' ').unwrap_or((&line, ""));
                        let data = serde_json::from_str(data).ok();
                        match cx.build_action(&format!("keyprompter::{name}"), data) {
                            Ok(action) => cx.dispatch_action(&*action),
                            Err(e) => view.update(cx, |v, cx| v.show_error(e.to_string(), cx)),
                        }
                    }
                });
            }
        })
        .detach();
        cx.activate(true);
    });
}
