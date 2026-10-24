use std::path::PathBuf;

use gpui_kit::*;
use keyprompter::{
    frame::{self, Frame},
    view::StageView,
};

const DEFAULT_H: f32 = 150.0;

fn main() {
    let source = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/stream.md"));
    let frame_path = frame::state_path();

    application().run(move |cx| {
        init(cx);
        let saved = frame_path.as_deref().and_then(frame::load);
        let frame = saved.unwrap_or_else(|| Frame {
            x: 0.0,
            y: 0.0,
            w: cx
                .primary_display()
                .map_or(1440.0, |d| f32::from(d.bounds().size.width)),
            h: DEFAULT_H,
        });
        open_window(
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
            |window, cx| cx.new(|cx| StageView::new(source, frame_path, window, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
