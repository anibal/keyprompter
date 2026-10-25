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
    motion::{Before, Motion},
    platform,
    rundown::Rundown,
    show::{Command, Show, format_elapsed},
    snap::{self, Drag, Gesture, Placement, Rect, Side, Widths},
    stage::{ease, slot},
    state::{self, Capture, State},
};

const COLUMN_W: f32 = 200.0;
const CARD_VPAD: f32 = 12.0;
const CARD_PAD: f32 = 10.0;

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

/// The open rundown was replaced or changed, so its undo state may differ.
pub struct DocumentChanged;

impl EventEmitter<DocumentChanged> for StageView {}

/// The strip stuck to another side, or came loose.
pub struct PlacementChanged;

impl EventEmitter<PlacementChanged> for StageView {}

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
    motion: Motion<CardLayout>,
    /// The card a right-click menu is open for.
    menu_target: Option<usize>,
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
            motion: Motion::default(),
            menu_target: None,
            layout_key: None,
            content_h: 0.0,
            saved,
            state_path,
            settling: None,
            _subscriptions: subscriptions,
            _tick: tick,
        };
        // macOS pulls a window under the menu bar when it first shows it, so
        // the saved placement goes back on once it is on screen.
        cx.on_next_frame(window, |this, window, cx| {
            let saved = this.saved.placement.as_ref();
            match saved.and_then(|p| p.frame_on(&platform::screens())) {
                Some(frame) => place(frame, window, cx),
                None => this.moved(window, cx),
            }
        });
        view
    }

    fn widths(&self) -> Widths {
        self.saved
            .placement
            .as_ref()
            .map_or(Widths::default(), |p| p.widths)
    }

    /// The side the strip is stuck to, as last saved.
    pub fn stuck(&self) -> Option<Side> {
        self.saved.placement.as_ref().and_then(|p| p.stick.side())
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
        self.persist(window, cx);
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

    pub fn toggle_capture(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.capture = match self.capture {
            Capture::Hidden => Capture::Shown,
            Capture::Shown => Capture::Hidden,
        };
        platform::hide_from_capture(window, self.capture == Capture::Hidden);
        self.persist(window, cx);
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
        self.motion.clear();
        self.layouts.clear();
        self.layout_key = None;
        self.error = None;
        self.persist(window, cx);
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// Applies a card command to the card in the hot spot.
    pub fn edit(&mut self, op: EditCard, cx: &mut Context<Self>) {
        self.edit_at(self.show.cursor(), op, cx);
    }

    /// Applies a card command to the card at `target`. The hot spot stays on
    /// its card unless that card is the one changed.
    pub fn edit_at(&mut self, target: usize, op: EditCard, cx: &mut Context<Self>) {
        self.change(
            |d, at| d.edit(at, target, op.title(), |card, side| op.edit(card, side)),
            cx,
        );
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.change(Document::undo, cx);
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.change(Document::redo, cx);
    }

    /// Applies `change` at the current card, animates from the old deck to the
    /// new one, saves, and shows the card it focuses.
    fn change(
        &mut self,
        change: impl FnOnce(&mut Document, usize) -> Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let cursor = self.show.cursor();
        let Some(document) = &mut self.document else {
            return;
        };
        let before = document.deck().slides.clone();
        let Some(focus) = change(document, cursor) else {
            return;
        };
        let after = &document.deck().slides;
        let ids: Vec<_> = after.iter().map(|s| s.id).collect();
        let layouts = std::mem::take(&mut self.layouts);
        let lines = |i: usize| layouts.get(i).map_or(0, |l: &CardLayout| l.body.len());
        // New text glows from where the old text ended. A card that only
        // changed section glows its whole body, since nothing else moves.
        let changed: Vec<_> = after
            .iter()
            .filter_map(|s| {
                let (i, b) = before.iter().enumerate().find(|(_, b)| b.id == s.id)?;
                if b.cue != s.cue || b.body != s.body {
                    Some((s.id, lines(i)))
                } else {
                    (b.section != s.section).then_some((s.id, 0))
                }
            })
            .collect();
        // A merge, or its redo, removes a card and changes the one before it.
        let into = before
            .windows(2)
            .find(|w| !ids.contains(&w[1].id) && changed.iter().any(|&(id, _)| id == w[0].id))
            .map(|w| w[0].id);
        let mut layouts = layouts.into_iter();
        let before = before
            .iter()
            .map(|slide| Before {
                id: slide.id,
                card: layouts.next().unwrap_or_default(),
            })
            .collect();
        self.motion.change(before, &ids, &changed, into);
        self.settle(focus, cx);
    }

    /// After a change to the document, saves it and shows the card at `focus`.
    fn settle(&mut self, focus: usize, cx: &mut Context<Self>) {
        let Some(document) = &self.document else {
            return;
        };
        self.error = self
            .library
            .save(document.id(), document.rundown())
            .err()
            .map(|e| format!("Can't save: {e}"));
        self.command(Command::Goto(focus), cx);
        self.layout_key = None;
        cx.emit(DocumentChanged);
    }

    /// Offers the card edits for the right-clicked card in a native menu,
    /// marking that card while the menu is open.
    fn card_menu(
        &mut self,
        index: usize,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ns_view) = platform::ns_view(window) else {
            return;
        };
        self.menu_target = Some(index);
        cx.notify();
        let titles: Vec<_> = CARD_MENU
            .iter()
            .map(|item| item.map(EditCard::title))
            .collect();
        // A spawned task runs outside any GPUI update, which the menu's own
        // event loop needs.
        cx.spawn_in(window, async move |this, cx| {
            let chosen =
                platform::context_menu(&ns_view, &titles, (f32::from(at.x), f32::from(at.y)));
            this.update(cx, |v, cx| {
                v.menu_target = None;
                match chosen.and_then(|i| CARD_MENU[i]) {
                    Some(op) => v.edit_at(index, op, cx),
                    None => cx.notify(),
                }
            })
            .ok();
        })
        .detach();
    }

    pub fn show_error(&mut self, error: String, cx: &mut Context<Self>) {
        self.error = Some(error);
        cx.notify();
    }

    fn tick(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.persist(window, cx);
        cx.notify();
    }

    /// Once any drag ends, pulls the strip onto its magnets and saves the
    /// placement. Also runs at launch, so a restored frame settles the same way.
    pub fn moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settling.is_some() {
            return;
        }
        self.settling = Some(cx.spawn_in(window, async move |this, cx| {
            while platform::pointer().down {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }
            this.update_in(cx, |this, window, cx| {
                this.settling = None;
                this.snap(window, cx);
                this.persist(window, cx);
            })
            .ok();
        }));
    }

    /// Pulls the frame onto its magnets. A move step with the pointer where it
    /// is leaves only the magnets to act.
    fn snap(&self, window: &Window, cx: &mut Context<Self>) {
        let Some(frame) = platform::frame(window) else {
            return;
        };
        let screens = platform::screens();
        let at = platform::pointer().at;
        let drag = Drag {
            gesture: Gesture::Move,
            grab: frame,
            at,
            symmetric: false,
        };
        let settled = snap::release(drag.step(at, false, &screens), &screens);
        if settled != frame {
            place(settled, window, cx);
        }
    }

    /// Classifies the live frame, records the width, saves when changed, and
    /// says when the stick changed. A test window has no frame and keeps the
    /// saved placement.
    fn persist(&mut self, window: &Window, cx: &mut Context<Self>) {
        let placement = platform::frame(window)
            .and_then(|f| snap::classify(f, &platform::screens(), self.widths()))
            .or_else(|| self.saved.placement.clone());
        let current = State {
            placement,
            rundown: self.document.as_ref().map(|d| d.id().to_owned()),
            capture: self.capture,
            text: self.text_step,
        };
        if self.saved != current
            && let Some(path) = &self.state_path
            && state::save(path, &current).is_ok()
        {
            let was = self.stuck();
            self.saved = current;
            if self.stuck() != was {
                cx.emit(PlacementChanged);
            }
        }
    }

    /// Puts the strip on the very top edge of a screen, over its menu bar,
    /// keeping its stick. `None` means its current screen.
    pub fn move_to_top(
        &mut self,
        screen: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(frame) = platform::frame(window) else {
            return;
        };
        let screens = platform::screens();
        let target = match screen {
            Some(i) => screens.get(i),
            None => snap::holding(frame, &screens),
        };
        if let Some(target) = target
            && let Some(placement) = snap::classify(frame, &screens, self.widths())
        {
            let placed = Placement {
                top: 0.0,
                ..placement
            };
            place(placed.on(target), window, cx);
        }
    }

    /// View > Stick to `side`: the strip flush there, at the width it last
    /// had on that side, else its current width.
    pub fn stick_to(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let Some(frame) = platform::frame(window) else {
            return;
        };
        let screens = platform::screens();
        if let Some(screen) = snap::holding(frame, &screens)
            && let Some(placement) = snap::classify(frame, &screens, self.widths())
        {
            place(placement.stuck(side, frame, screen), window, cx);
        }
    }

    /// Puts the strip at a screen-local `[x, top, w, h]` on the main screen.
    pub fn set_frame(
        &mut self,
        [x, top, w, h]: [f32; 4],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let screens = platform::screens();
        let Some(main) = screens.first() else {
            return;
        };
        let s = main.frame;
        let frame = Rect {
            x: s.x + x,
            y: s.y + s.h - top - h,
            w,
            h,
        };
        place(frame, window, cx);
    }

    pub fn command(&mut self, cmd: Command, cx: &mut Context<Self>) {
        self.show.apply(cmd, self.slides().len(), Instant::now());
        cx.notify();
    }
}

/// Moves the strip to `frame` once this update ends. AppKit reports the new
/// size to GPUI on the spot, but that report needs the window, which the
/// update still holds, so the viewport is refreshed by hand. That also runs
/// the bounds observers, which settle and save.
fn place(frame: Rect, window: &Window, cx: &mut App) {
    window.defer(cx, move |window, cx| {
        platform::set_frame(window, frame);
        window.bounds_changed(cx);
    });
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

/// Where cards go on the strip and how big their text is.
struct Geometry {
    hot_x: f32,
    stage_w: f32,
    card_w: f32,
    card_h: f32,
    win_h: f32,
    scale: TypeScale,
    content_h: f32,
}

/// How one card is drawn this frame.
struct Appearance {
    /// Distance from the hot spot, in slide indices.
    d: f32,
    opacity: f32,
    /// Extra scale on top of the card's slot, for cards coming and going.
    shrink: f32,
    /// Body lines from the first number on glow at the second's strength.
    glow: (usize, f32),
    /// A menu is open for this card, so it is outlined like the current one.
    marked: bool,
}

/// A card at its slot, or `None` when it is off the strip or invisible.
fn card(
    g: &Geometry,
    layout: &CardLayout,
    look: Appearance,
    t: &Theme,
    window: &Window,
) -> Option<Div> {
    let s = slot(look.d);
    let size = s.scale * look.shrink;
    let opacity = s.opacity * look.opacity;
    let (w, h) = (g.card_w * size, g.card_h * size);
    let center = g.hot_x + s.x * g.card_w;
    if opacity <= 0.0 || center + w / 2.0 < 0.0 || center - w / 2.0 > g.stage_w {
        return None;
    }
    let scale = g.scale;
    let cue_h = scale.cue * LINE_HEIGHT * size;
    let body_h = scale.body * LINE_HEIGHT * size;
    let fit = layout.fit(scale, g.content_h);
    let cue = layout.cue[..fit.cue].iter().map(|line| {
        div()
            .h(px(cue_h))
            .line_height(px(cue_h))
            .text_size(px(scale.cue * size))
            .font_weight(FontWeight::BOLD)
            .whitespace_nowrap()
            .child(line.clone())
    });
    let (glow_from, glow) = look.glow;
    let body = layout.body[..fit.body].iter().enumerate().map(|(j, line)| {
        let marker = match line.mark {
            Mark::None => None,
            Mark::Bullet => Some("•"),
            Mark::Hang => Some(""),
        };
        let lit = if j >= glow_from { 0.3 * glow } else { 0.0 };
        div()
            .flex()
            .h(px(body_h))
            .line_height(px(body_h))
            .text_size(px(scale.body * size))
            .whitespace_nowrap()
            .rounded(px(4.0 * size))
            .bg(t.accent.opacity(lit))
            .children(marker.map(|m| {
                div()
                    .w(px(scale.indent() * size))
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
                    .px(px(6.0 * size))
                    .rounded(px(6.0 * size))
                    .bg(t.error)
                    .text_color(t.bg.opacity(1.0))
                    .text_size(px(scale.body * 0.75 * size))
                    .line_height(px(scale.body * 1.1 * size))
                    .font_weight(FontWeight::BOLD)
                    .child(format!("+{} lines", fit.hidden)),
            )
    });
    Some(
        div()
            .absolute()
            .left(px(center - w / 2.0))
            .top(px((g.win_h - h) / 2.0))
            .w(px(w))
            .h(px(h))
            .p(px(CARD_PAD * size))
            .overflow_hidden()
            .rounded(px(10.0 * size))
            .border_2()
            .border_color(t.accent.opacity(if look.marked {
                1.0
            } else {
                (1.0 - look.d.abs()).max(0.0)
            }))
            .bg(t.card)
            .opacity(opacity)
            .text_color(t.fg)
            .flex()
            .flex_col()
            .children(cue)
            .children(body)
            .children(more),
    )
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
        let slide = !platform::reduce_motion();
        self.pos = if slide {
            ease(self.pos, target, dt)
        } else {
            target
        };
        let ids: Vec<_> = self.slides().iter().map(|s| s.id).collect();
        let moving = self.motion.step(&ids, dt, slide) || self.pos != target;
        self.last_frame = moving.then_some(now);
        if moving {
            window.request_animation_frame();
        }

        let t = theme(window.appearance());
        let view = window.viewport_size();
        let (win_w, win_h) = (f32::from(view.width), f32::from(view.height));
        let stage_w = win_w - COLUMN_W;
        let card_w = snap::card_width(win_w);
        // Read live, so the current card holds still on screen while the
        // window slides under it.
        let screens = platform::screens();
        let hot_x = platform::frame(window)
            .and_then(|f| {
                let screen = snap::holding(f, &screens)?.frame;
                Some(snap::hot_spot(f, screen, COLUMN_W, card_w))
            })
            .unwrap_or(win_w / 2.0 - COLUMN_W);
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

        let geometry = Geometry {
            hot_x,
            stage_w,
            card_w,
            card_h,
            win_h,
            scale,
            content_h,
        };
        let ghosts = self.motion.ghosts().iter().filter_map(|ghost| {
            let shrink = if ghost.into.is_some() {
                1.0
            } else {
                0.7 + 0.3 * ghost.fade
            };
            let look = Appearance {
                d: ghost.rank - self.pos,
                opacity: ghost.fade,
                shrink,
                glow: (0, 0.0),
                marked: false,
            };
            card(&geometry, &ghost.card, look, &t, window).map(IntoElement::into_any_element)
        });
        let live = self
            .layouts
            .iter()
            .zip(self.slides())
            .enumerate()
            .filter_map(|(i, (layout, slide))| {
                let motion = self.motion.look(slide.id, i);
                let look = Appearance {
                    d: motion.rank - self.pos,
                    opacity: motion.appear,
                    shrink: 0.8 + 0.2 * motion.appear,
                    glow: (motion.glow_from, motion.glow),
                    marked: self.menu_target == Some(i),
                };
                let element = card(&geometry, layout, look, &t, window)?
                    .id(("slide", i))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            // The current card lets the press through to
                            // the strip, which drags the window.
                            if i != this.show.cursor() {
                                cx.stop_propagation();
                                cx.activate(true);
                                this.command(Command::Goto(i), cx);
                            }
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                            this.card_menu(i, e.position, window, cx)
                        }),
                    );
                Some(element.into_any_element())
            });
        let cards: Vec<_> = ghosts.chain(live).collect();

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
