use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use gpui_kit::*;

use crate::{
    app::{self, CARD_MENU, EditCard},
    deck::{Slide, Span},
    document::Document,
    layout::{self, CardLayout, LINE_HEIGHT, Mark, TypeScale},
    library::Library,
    platform,
    rundown::Rundown,
    show::{Command, Show, format_elapsed},
    stage::{ease, slot},
    state::{self, Capture, Frame, State},
};

const COLUMN_W: f32 = 200.0;
const MAX_CARD_W: f32 = 420.0;
const CARD_VPAD: f32 = 12.0;
const CARD_PAD: f32 = 10.0;
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
    document: Option<Document>,
    library: Library,
    error: Option<String>,
    state_path: Option<PathBuf>,
    saved: State,
    capture: Capture,
    text_step: i32,
    layouts: Vec<CardLayout>,
    /// What `layouts` were laid out for. `None` once the deck has changed.
    layout_key: Option<(TypeScale, f32)>,
    content_h: f32,
    settling: Option<Task<()>>,
    _subscriptions: [Subscription; 3],
    _tick: Task<()>,
}

impl StageView {
    pub fn new(
        document: Option<Document>,
        library: Library,
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
        let view = Self {
            focus,
            show: Show::default(),
            pos: 0.0,
            last_frame: None,
            document,
            library,
            error: None,
            capture: saved.capture,
            text_step: saved.text.clamp(layout::MIN_STEP, layout::MAX_STEP),
            layouts: Vec::new(),
            layout_key: None,
            content_h: 0.0,
            saved,
            state_path,
            settling: None,
            _subscriptions: subscriptions,
            _tick: tick,
        };
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

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn text_step(&self) -> i32 {
        self.text_step
    }

    pub fn set_text_step(&mut self, step: i32, window: &Window, cx: &mut Context<Self>) {
        self.text_step = step.clamp(layout::MIN_STEP, layout::MAX_STEP);
        self.persist(window);
        cx.notify();
    }

    /// The cards that don't fit, as `(slide index, hidden lines)`.
    pub fn overflowing(&self) -> Vec<(usize, usize)> {
        let Some((scale, _)) = self.layout_key else {
            return Vec::new();
        };
        self.layouts
            .iter()
            .enumerate()
            .map(|(i, card)| (i, card.fit(scale, self.content_h).hidden))
            .filter(|&(_, hidden)| hidden > 0)
            .collect()
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

    pub fn document(&self) -> Option<&Document> {
        self.document.as_ref()
    }

    fn slides(&self) -> &[Slide] {
        self.document.as_ref().map_or(&[], |d| &d.deck().slides)
    }

    /// Switches to another rundown, starting it from the first slide.
    pub fn open(&mut self, id: String, rundown: Rundown, window: &Window, cx: &mut Context<Self>) {
        self.document = Some(Document::new(id, rundown));
        self.show = Show::default();
        self.layout_key = None;
        self.error = None;
        self.persist(window);
        cx.notify();
    }

    /// Applies an edit to the current card, saves, and shows the card it focuses.
    pub fn edit(&mut self, op: EditCard, cx: &mut Context<Self>) {
        let cursor = self.show.cursor();
        let focus = self
            .document
            .as_mut()
            .and_then(|d| d.edit(cursor, |card| op.edit(card)));
        self.settle(focus, cx);
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let cursor = self.show.cursor();
        let focus = self.document.as_mut().and_then(|d| d.undo(cursor));
        self.settle(focus, cx);
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let cursor = self.show.cursor();
        let focus = self.document.as_mut().and_then(|d| d.redo(cursor));
        self.settle(focus, cx);
    }

    /// After a change to the document, saves it and shows the card at `focus`.
    fn settle(&mut self, focus: Option<usize>, cx: &mut Context<Self>) {
        let (Some(focus), Some(document)) = (focus, &self.document) else {
            return;
        };
        self.error = self
            .library
            .save(document.id(), document.rundown())
            .err()
            .map(|e| format!("Can't save: {e}"));
        self.command(Command::Goto(focus), cx);
        self.layout_key = None;
    }

    /// Makes the right-clicked card current, then offers the card edits in a
    /// native menu.
    fn card_menu(
        &mut self,
        index: usize,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command(Command::Goto(index), cx);
        let Some(ns_view) = platform::ns_view(window) else {
            return;
        };
        let titles: Vec<_> = CARD_MENU
            .iter()
            .map(|item| item.map(EditCard::title))
            .collect();
        // A spawned task runs outside any GPUI update, which the menu's own
        // event loop needs.
        cx.spawn_in(window, async move |this, cx| {
            let chosen =
                platform::context_menu(&ns_view, &titles, (f32::from(at.x), f32::from(at.y)));
            if let Some(op) = chosen.and_then(|i| CARD_MENU[i]) {
                this.update(cx, |v, cx| v.edit(op, cx)).ok();
            }
        })
        .detach();
    }

    pub fn show_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        cx.notify();
    }

    fn tick(&mut self, window: &Window, cx: &mut Context<Self>) {
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
            rundown: self.document.as_ref().map(|d| d.id().to_owned()),
            capture: self.capture,
            text: self.text_step,
        };
        if self.saved != current
            && let Some(path) = &self.state_path
            && state::save(path, &current).is_ok()
        {
            self.saved = current;
        }
    }

    pub fn command(&mut self, cmd: Command, cx: &mut Context<Self>) {
        self.show.apply(cmd, self.slides().len(), Instant::now());
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
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    StyledText::new(text).with_runs(layout::runs(
        spans,
        &window.text_style().font(),
        color,
        code_bg,
    ))
}

impl Render for StageView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

        let content_h = card_h - 2.0 * CARD_PAD;
        let scale = TypeScale::new(content_h, self.text_step);
        self.content_h = content_h;
        let key = (scale, card_w);
        if self.layout_key != Some(key) {
            self.layouts = self
                .slides()
                .iter()
                .map(|slide| layout::layout(slide, scale, card_w - 2.0 * CARD_PAD, window))
                .collect();
            self.layout_key = Some(key);
        }

        let cards: Vec<_> = self
            .layouts
            .iter()
            .enumerate()
            .filter_map(|(i, card)| {
                let d = i as f32 - self.pos;
                let s = slot(d);
                let (w, h) = (card_w * s.scale, card_h * s.scale);
                let center = hot_x + s.x * card_w;
                if s.opacity <= 0.0 || center + w / 2.0 < 0.0 || center - w / 2.0 > stage_w {
                    return None;
                }
                let cue_h = scale.cue * LINE_HEIGHT * s.scale;
                let body_h = scale.body * LINE_HEIGHT * s.scale;
                let fit = card.fit(scale, content_h);
                let cue = card.cue[..fit.cue].iter().map(|line| {
                    div()
                        .h(px(cue_h))
                        .line_height(px(cue_h))
                        .text_size(px(scale.cue * s.scale))
                        .font_weight(FontWeight::BOLD)
                        .whitespace_nowrap()
                        .child(line.clone())
                });
                let body = card.body[..fit.body].iter().map(|line| {
                    let marker = match line.mark {
                        Mark::None => None,
                        Mark::Bullet => Some("•"),
                        Mark::Hang => Some(""),
                    };
                    div()
                        .flex()
                        .h(px(body_h))
                        .line_height(px(body_h))
                        .text_size(px(scale.body * s.scale))
                        .whitespace_nowrap()
                        .children(marker.map(|m| {
                            div()
                                .w(px(scale.indent() * s.scale))
                                .flex_none()
                                .text_color(t.accent)
                                .child(m)
                        }))
                        .child(rich(&line.spans, t.fg, t.code_bg, window))
                });
                let more = (fit.hidden > 0).then(|| {
                    div()
                        .flex()
                        .justify_end()
                        .items_center()
                        .h(px(body_h))
                        .child(
                            div()
                                .px(px(6.0 * s.scale))
                                .rounded(px(6.0 * s.scale))
                                .bg(t.error)
                                .text_color(t.bg.opacity(1.0))
                                .text_size(px(scale.body * 0.75 * s.scale))
                                .line_height(px(scale.body * 1.1 * s.scale))
                                .font_weight(FontWeight::BOLD)
                                .child(format!("+{} lines", fit.hidden)),
                        )
                });
                Some(
                    div()
                        .id(("slide", i))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                this.card_menu(i, e.position, window, cx)
                            }),
                        )
                        .absolute()
                        .left(px(center - w / 2.0))
                        .top(px((win_h - h) / 2.0))
                        .w(px(w))
                        .h(px(h))
                        .p(px(CARD_PAD * s.scale))
                        .overflow_hidden()
                        .rounded(px(10.0 * s.scale))
                        .border_2()
                        .border_color(t.accent.opacity((1.0 - d.abs()).max(0.0)))
                        .bg(t.card)
                        .opacity(s.opacity)
                        .text_color(t.fg)
                        .flex()
                        .flex_col()
                        .children(cue)
                        .children(body)
                        .children(more),
                )
            })
            .collect();

        let slides = self.slides();
        let title = self
            .document
            .as_ref()
            .map_or_else(String::new, |d| d.rundown().title.clone());
        let section = slides
            .get(self.show.cursor())
            .and_then(|s| s.section.clone())
            .unwrap_or_default();
        let position =
            (!slides.is_empty()).then(|| format!("{} / {}", self.show.cursor() + 1, slides.len()));
        let overflow_note = match self.overflowing().as_slice() {
            [] => None,
            [(i, _)] => Some(format!("Card {} doesn't fit", i + 1)),
            many => Some(format!("{} cards don't fit", many.len())),
        };
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
                    .child(title),
            )
            .children(overflow_note.map(|note| {
                div()
                    .text_size(px(11.0))
                    .text_color(t.error)
                    .truncate()
                    .child(note)
            }))
            .children(self.error.as_ref().map(|e| {
                div()
                    .text_size(px(11.0))
                    .text_color(t.error)
                    .truncate()
                    .child(e.clone())
            }));

        let empty = slides.is_empty().then(|| {
            let hint = if self.document.is_some() {
                "This rundown has no cards."
            } else {
                "Import a Markdown rundown with File > Import Markdown… (⌘O)."
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
            .key_context(app::STRIP)
            .track_focus(&self.focus)
            // The strip is a non-activating panel, so a click alone would leave the
            // previous app's menu bar showing.
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.activate(true);
                window.start_window_move();
            })
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
