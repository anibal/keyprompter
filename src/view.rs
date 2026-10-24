use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use gpui_kit::*;

use crate::{
    deck::{Block, Span, SpanStyle, parse},
    platform,
    show::{Command, Show, format_elapsed},
    stage::{ease, slot},
    state::{self, Capture, Frame, State},
};

const COLUMN_W: f32 = 200.0;
const MAX_CARD_W: f32 = 420.0;
const CARD_VPAD: f32 = 12.0;
const SNAP_REACH: f32 = 24.0;

struct Theme {
    bg: Hsla,
    fg: Hsla,
    dim: Hsla,
    card: Hsla,
    accent: Hsla,
    code_bg: Hsla,
    error: Hsla,
}

fn theme(appearance: WindowAppearance) -> Theme {
    let c = |hex: u32| -> Hsla { rgba(hex).into() };
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Theme {
            bg: c(0x07090dc0),
            fg: c(0xf2f4f8ff),
            dim: c(0xf2f4f899),
            card: c(0xffffff12),
            accent: c(0x4cc2ffff),
            code_bg: c(0xffffff22),
            error: c(0xff7b72ff),
        },
        WindowAppearance::Light | WindowAppearance::VibrantLight => Theme {
            bg: c(0xffffffc0),
            fg: c(0x14171cff),
            dim: c(0x14171c99),
            card: c(0x00000010),
            accent: c(0x0a6cffff),
            code_bg: c(0x00000018),
            error: c(0xc62828ff),
        },
    }
}

pub struct StageView {
    focus: FocusHandle,
    show: Show,
    pos: f32,
    last_frame: Option<Instant>,
    source: Option<PathBuf>,
    mtime: Option<SystemTime>,
    error: Option<String>,
    state_path: Option<PathBuf>,
    saved: State,
    capture: Capture,
    settling: Option<Task<()>>,
    _subscriptions: [Subscription; 3],
    _tick: Task<()>,
}

impl StageView {
    pub fn new(
        source: Option<PathBuf>,
        saved: State,
        state_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        platform::hide_from_capture(window, saved.capture == Capture::Hidden);
        platform::allow_over_menu_bar(window);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let subscriptions = [
            cx.observe_window_activation(window, |this, window, cx| window.focus(&this.focus, cx)),
            cx.observe_window_appearance(window, |_, _, cx| cx.notify()),
            cx.observe_window_bounds(window, |this, window, cx| this.moved(window, cx)),
        ];
        let tick = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update_in(cx, |this, window, cx| this.tick(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut view = Self {
            focus,
            show: Show::new(Default::default()),
            pos: 0.0,
            last_frame: None,
            source,
            mtime: None,
            error: None,
            capture: saved.capture,
            saved,
            state_path,
            settling: None,
            _subscriptions: subscriptions,
            _tick: tick,
        };
        view.reload_if_changed();
        // macOS pulls a window under the menu bar when it first shows it, so
        // the saved frame goes back on once it is on screen.
        cx.on_next_frame(window, |this, window, cx| {
            if let Some(frame) = this.saved.frame {
                platform::restore(window, frame);
            }
            this.moved(window, cx);
        });
        view
    }

    pub fn cursor(&self) -> usize {
        self.show.cursor()
    }

    pub fn pos(&self) -> f32 {
        self.pos
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn capture(&self) -> Capture {
        self.capture
    }

    pub fn toggle_capture(&mut self, window: &Window) {
        self.capture = match self.capture {
            Capture::Hidden => Capture::Shown,
            Capture::Shown => Capture::Hidden,
        };
        platform::hide_from_capture(window, self.capture == Capture::Hidden);
        self.persist(window);
    }

    /// Switches to another rundown, starting it from the first slide.
    pub fn open(&mut self, source: PathBuf, window: &Window, cx: &mut Context<Self>) {
        self.source = Some(source);
        self.mtime = None;
        self.show = Show::new(Default::default());
        self.reload_if_changed();
        self.persist(window);
        cx.notify();
    }

    fn tick(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.reload_if_changed();
        self.persist(window);
        cx.notify();
    }

    /// Once any drag ends, snaps onto nearby edges and saves the frame. Also
    /// runs at launch, so a restored frame settles the same way.
    pub fn moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settling.is_some() {
            return;
        }
        self.settling = Some(cx.spawn_in(window, async move |this, cx| {
            while platform::mouse_down() {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
            this.update_in(cx, |this, window, _| {
                this.settling = None;
                platform::snap_to_edges(window, SNAP_REACH);
                this.persist(window);
            })
            .ok();
        }));
    }

    fn reload_if_changed(&mut self) {
        let Some(source) = &self.source else {
            return;
        };
        let loaded = fs::metadata(source)
            .and_then(|m| m.modified())
            .and_then(|modified| {
                if Some(modified) == self.mtime {
                    return Ok(None);
                }
                Ok(Some((modified, fs::read_to_string(source)?)))
            });
        match loaded {
            Ok(None) => {}
            Ok(Some((modified, text))) => {
                self.mtime = Some(modified);
                self.error = None;
                self.show
                    .apply(Command::Reload(parse(&text)), Instant::now());
            }
            Err(e) => self.error = Some(format!("Can't read file: {e}")),
        }
    }

    fn persist(&mut self, window: &Window) {
        let b = window.bounds();
        let frame = platform::frame(window).unwrap_or(Frame {
            x: f32::from(b.origin.x),
            y: f32::from(b.origin.y),
            w: f32::from(b.size.width),
            h: f32::from(b.size.height),
        });
        let current = State {
            frame: Some(frame),
            deck: self.source.clone(),
            capture: self.capture,
        };
        if self.saved != current
            && let Some(path) = &self.state_path
            && state::save(path, &current).is_ok()
        {
            self.saved = current;
        }
    }

    pub fn command(&mut self, cmd: Command, cx: &mut Context<Self>) {
        self.show.apply(cmd, Instant::now());
        cx.notify();
    }
}

fn wall_clock() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as libc::time_t);
    // SAFETY: localtime_r only writes into the zeroed `tm` we own.
    let tm = unsafe {
        let mut tm = std::mem::zeroed();
        libc::localtime_r(&secs, &mut tm);
        tm
    };
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

fn rich(spans: &[Span], color: Hsla, code_bg: Hsla, window: &Window) -> StyledText {
    let base = window.text_style().font();
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    let runs = spans
        .iter()
        .map(|s| {
            let mut font = base.clone();
            let mut background_color = None;
            match s.style {
                SpanStyle::Plain => {}
                SpanStyle::Bold => font.weight = FontWeight::BOLD,
                SpanStyle::Italic => font.style = FontStyle::Italic,
                SpanStyle::Code => {
                    font.family = "Menlo".into();
                    background_color = Some(code_bg);
                }
            }
            TextRun {
                len: s.text.len(),
                font,
                color,
                background_color,
                underline: None,
                strikethrough: None,
            }
        })
        .collect();
    StyledText::new(text).with_runs(runs)
}

impl Render for StageView {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        let target = self.show.cursor() as f32;
        let dt = self
            .last_frame
            .map_or(0.0, |t| (now - t).as_secs_f32().min(0.05));
        self.pos = if platform::reduce_motion() {
            target
        } else {
            ease(self.pos, target, dt)
        };
        self.last_frame = (self.pos != target).then_some(now);
        if self.pos != target {
            window.request_animation_frame();
        }

        let t = theme(window.appearance());
        let view = window.viewport_size();
        let (win_w, win_h) = (f32::from(view.width), f32::from(view.height));
        let stage_w = win_w - COLUMN_W;
        let hot_x = win_w / 2.0 - COLUMN_W;
        let card_w = MAX_CARD_W.min(win_w * 0.3);
        let card_h = win_h - CARD_VPAD;

        let cards: Vec<_> = self
            .show
            .slides()
            .iter()
            .enumerate()
            .filter_map(|(i, slide)| {
                let d = i as f32 - self.pos;
                let s = slot(d);
                let (w, h) = (card_w * s.scale, card_h * s.scale);
                let center = hot_x + s.x * card_w;
                if s.opacity <= 0.0 || center + w / 2.0 < 0.0 || center - w / 2.0 > stage_w {
                    return None;
                }
                let pad = 10.0 * s.scale;
                let body_px = 14.0 * s.scale;
                let blocks = slide.body.iter().map(|block| match block {
                    Block::Bullet(spans) => div()
                        .flex()
                        .gap(px(6.0 * s.scale))
                        .child(div().text_color(t.accent).child("•"))
                        .child(div().flex_1().child(rich(spans, t.fg, t.code_bg, window))),
                    Block::Para(spans) => div().child(rich(spans, t.fg, t.code_bg, window)),
                });
                Some(
                    div()
                        .id(("slide", i))
                        .absolute()
                        .left(px(center - w / 2.0))
                        .top(px((win_h - h) / 2.0))
                        .w(px(w))
                        .h(px(h))
                        .p(px(pad))
                        .overflow_hidden()
                        .rounded(px(10.0 * s.scale))
                        .border_2()
                        .border_color(t.accent.opacity((1.0 - d.abs()).max(0.0)))
                        .bg(t.card)
                        .opacity(s.opacity)
                        .text_size(px(body_px))
                        .text_color(t.fg)
                        .flex()
                        .flex_col()
                        .gap(px(3.0 * s.scale))
                        .child(
                            div()
                                .text_size(px(20.0 * s.scale))
                                .font_weight(FontWeight::BOLD)
                                .child(slide.cue.clone()),
                        )
                        .children(blocks),
                )
            })
            .collect();

        let file_name = self
            .source
            .as_deref()
            .and_then(|s| s.file_name())
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let section = self
            .show
            .current()
            .and_then(|s| s.section.clone())
            .unwrap_or_default();
        let position = (!self.show.slides().is_empty())
            .then(|| format!("{} / {}", self.show.cursor() + 1, self.show.slides().len()));
        let column = div()
            .w(px(COLUMN_W))
            .h_full()
            .flex_none()
            .px_3()
            .flex()
            .flex_col()
            .justify_center()
            .gap_0p5()
            .border_r_1()
            .border_color(t.card)
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::BOLD)
                    .truncate()
                    .child(section),
            )
            .children(position.map(|p| div().text_size(px(13.0)).child(p)))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_size(px(13.0))
                    .text_color(t.dim)
                    .child(wall_clock())
                    .child(format_elapsed(self.show.elapsed(now))),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(t.dim)
                    .truncate()
                    .child(file_name),
            )
            .children(self.error.as_ref().map(|e| {
                div()
                    .text_size(px(11.0))
                    .text_color(t.error)
                    .truncate()
                    .child(e.clone())
            }));

        let empty = self.show.slides().is_empty().then(|| {
            let hint = if self.source.is_some() {
                "No slides yet. Add `## cue` headings to the file."
            } else {
                "Open a rundown with ⌘O."
            };
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(t.dim)
                .child(hint)
        });

        div()
            .id("root")
            .track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .size_full()
            .flex()
            .bg(t.bg)
            .text_color(t.fg)
            .child(column)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .relative()
                    .overflow_hidden()
                    .children(empty)
                    .children(cards),
            )
    }
}
