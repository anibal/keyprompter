use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use gpui_kit::{
    component::{
        Sizable,
        input::{self as text, Input, InputEvent, InputState, Textarea, TextareaState},
    },
    *,
};

use crate::{
    app::{self, CARD_MENU, CancelEdit, CardItem, EditCard},
    deck::{Slide, Span},
    document::Document,
    layout::{self, CardLayout, LINE_HEIGHT, Mark, TypeScale},
    library::Library,
    motion::{Before, Motion},
    platform,
    rundown::{CardId, Edit, Rundown},
    show::{Command, Show, format_elapsed},
    snap::{self, Drag, Edge, Gesture, Lean, Placement, Rect, Side, Stick, Widths},
    stage::{Stage, ease, label_span, slot},
    state::{self, Capture, State},
};

const COLUMN_W: f32 = 200.0;
const CARD_VPAD: f32 = 12.0;
const CARD_PAD: f32 = 10.0;
const RAIL_GAP: f32 = 18.0;
const HEADER_LABEL_H: f32 = 14.0;
const HEADER_LABEL_MIN_W: f32 = 48.0;

/// A section's run of cards: its title, and which run with that title it is.
type RunKey = (String, usize);

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

enum Field {
    Cue,
    Body,
}

/// The open rundown was replaced or changed, so its undo state may differ.
pub struct DocumentChanged;

impl EventEmitter<DocumentChanged> for StageView {}

/// The strip stuck to another side, or came loose.
pub struct PlacementChanged;

impl EventEmitter<PlacementChanged> for StageView {}

/// A card whose text is open for editing, in two fields styled like the card.
struct Editing {
    card: CardId,
    cue: Entity<InputState>,
    body: Entity<TextareaState>,
    _subscriptions: [Subscription; 2],
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
    motion: Motion<CardLayout>,
    /// Where each section header's line ends, in ranks, easing toward its
    /// cards so the line glides when a card changes section without moving.
    headers: HashMap<RunKey, (f32, f32)>,
    /// The card a right-click menu is open for.
    menu_target: Option<usize>,
    /// What `layouts` were laid out for. `None` once the deck has changed.
    layout_key: Option<(TypeScale, f32)>,
    content_h: f32,
    editing: Option<Editing>,
    /// The gesture in flight. While one lives it owns the frame.
    drag: Option<Drag>,
    /// Moves `place` has queued that haven't landed yet.
    placing: usize,
    _drag_task: Option<Task<()>>,
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
            cx.observe_window_activation(window, |this, window, cx| {
                // An open edit keeps the focus while the window is active and
                // is saved when it is not.
                if window.is_window_active() && this.editing.is_some() {
                    return;
                }
                this.finish(true, window, cx);
                window.focus(&this.focus, cx);
            }),
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
            headers: HashMap::new(),
            menu_target: None,
            editing: None,
            layout_key: None,
            content_h: 0.0,
            saved,
            state_path,
            drag: None,
            placing: 0,
            _drag_task: None,
            _subscriptions: subscriptions,
            _tick: tick,
        };
        // macOS pulls a window under the menu bar when it first shows it, so
        // the saved placement goes back on once it is on screen.
        cx.on_next_frame(window, |this, window, cx| {
            let saved = this.saved.placement.as_ref();
            match saved.and_then(|p| p.frame_on(&platform::screens())) {
                Some(frame) => this.place(frame, window, cx),
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

    /// Each titled section's run of cards, with the ranks of its first and
    /// last card.
    fn section_runs(&self) -> Vec<(RunKey, (f32, f32))> {
        let mut runs: Vec<(RunKey, (f32, f32))> = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        let mut current = None;
        for (i, slide) in self.slides().iter().enumerate() {
            let rank = self.motion.look(slide.id, i).rank;
            if slide.section_start {
                current = slide.section.as_deref().map(|title| {
                    let n = seen.entry(title).or_insert(0);
                    runs.push(((title.to_string(), *n), (rank, rank)));
                    *n += 1;
                    runs.len() - 1
                });
            } else if let Some(run) = current {
                runs[run].1.1 = rank;
            }
        }
        runs
    }

    /// Eases the header lines toward the section runs. Returns whether any is
    /// still on its way.
    fn step_headers(&mut self, dt: f32, slide: bool) -> bool {
        let mut easing = false;
        let headers = self
            .section_runs()
            .into_iter()
            .map(|(key, target)| {
                let eased = match self.headers.get(&key) {
                    Some(&(first, last)) if slide => {
                        (ease(first, target.0, dt), ease(last, target.1, dt))
                    }
                    _ => target,
                };
                easing |= eased != target;
                (key, eased)
            })
            .collect();
        self.headers = headers;
        easing
    }

    /// Switches to another rundown, starting it from the first slide.
    pub fn open(
        &mut self,
        id: String,
        rundown: Rundown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.finish(true, window, cx);
        self.document = Some(Document::new(id, rundown));
        self.show = Show::default();
        self.motion.clear();
        self.headers.clear();
        self.layouts.clear();
        self.layout_key = None;
        self.error = None;
        self.persist(window, cx);
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// Applies a card command to the card in the hot spot.
    pub fn edit(&mut self, op: EditCard, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_at(self.show.cursor(), op, window, cx);
    }

    /// Applies a card command to the card at `target`, after saving any text
    /// edit. The hot spot stays on its card unless that card is the one changed.
    pub fn edit_at(
        &mut self,
        target: usize,
        op: EditCard,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.finish(true, window, cx);
        self.change(
            |d, at| d.edit(at, target, op.title(), |card, side| op.edit(card, side)),
            cx,
        );
    }

    /// While a card's text is open, Undo and Redo belong to the focused field.
    /// A card change reverted from under an open edit would leave it editing
    /// a card that is gone or different.
    pub fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            window.dispatch_action(text::Undo.boxed_clone(), cx);
        } else {
            self.change(Document::undo, cx);
        }
    }

    pub fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            window.dispatch_action(text::Redo.boxed_clone(), cx);
        } else {
            self.change(Document::redo, cx);
        }
    }

    pub fn editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Opens the card in the hot spot for editing.
    pub fn begin_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            return;
        }
        let Some(card) = self
            .document
            .as_ref()
            .and_then(|d| d.rundown().cards().nth(self.show.cursor()))
        else {
            return;
        };
        let (id, cue_text, body_text) = (card.id, card.cue.clone(), card.body.clone());
        let cue = cx.new(|cx| InputState::new(window, cx));
        let body = cx.new(|cx| TextareaState::new(window, cx));
        cue.update(cx, |state, cx| {
            state.set_value(cue_text, window, cx);
            state.focus(window, cx);
        });
        body.update(cx, |state, cx| state.set_value(body_text, window, cx));
        let subscriptions = [
            cx.subscribe_in(
                &cue,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter {
                        secondary: false, ..
                    } => this.focus_field(Field::Body, window, cx),
                    InputEvent::Blur => this.lost_focus(window, cx),
                    _ => {}
                },
            ),
            cx.subscribe_in(&body, window, |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::Blur = event {
                    this.lost_focus(window, cx);
                }
            }),
        ];
        self.editing = Some(Editing {
            card: id,
            cue,
            body,
            _subscriptions: subscriptions,
        });
        cx.emit(DocumentChanged);
        cx.notify();
    }

    /// Brings the card at `index` to the middle and opens it for editing.
    fn edit_text_at(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.command(Command::Goto(index), window, cx);
        self.begin_edit(window, cx);
    }

    /// The one way out of editing: saves the fields as a single undo step or
    /// drops them, and hands the focus back to the strip. An empty cue is not
    /// a card, so it reverts.
    fn finish(&mut self, save: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editing) = self.editing.take() else {
            return;
        };
        window.focus(&self.focus, cx);
        let cue = editing.cue.read(cx).value().to_string();
        let body = editing.body.read(cx).value().to_string();
        let target = self.document.as_ref().and_then(|d| {
            d.rundown()
                .card_ids()
                .iter()
                .position(|&i| i == editing.card)
        });
        if let (true, Some(target)) = (save && !cue.trim().is_empty(), target) {
            let card = editing.card;
            self.change(
                |d, at| {
                    d.edit(at, target, "Edit Card", |_, _| {
                        Edit::SetText(card, cue, body)
                    })
                },
                cx,
            );
        }
        cx.emit(DocumentChanged);
        cx.notify();
    }

    fn focus_field(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editing) = &self.editing else {
            return;
        };
        match field {
            Field::Cue => editing.cue.update(cx, |s, cx| s.focus(window, cx)),
            Field::Body => editing.body.update(cx, |s, cx| s.focus(window, cx)),
        }
    }

    /// A field lost the focus. Moving between the two fields is not leaving.
    fn lost_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The field is mid-update here, so the edit can't be dropped yet.
        cx.defer_in(window, |this, window, cx| {
            let Some(editing) = &this.editing else {
                return;
            };
            let inside = editing.cue.focus_handle(cx).is_focused(window)
                || editing.body.focus_handle(cx).is_focused(window);
            if !inside || !window.is_window_active() {
                this.finish(true, window, cx);
            }
        });
    }

    /// Moves between the fields with the arrow keys when the caret is at the
    /// edge of its field. `true` when it did.
    fn cross(&mut self, down: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(editing) = &self.editing else {
            return false;
        };
        let in_cue = editing.cue.focus_handle(cx).is_focused(window);
        let to = match (in_cue, down) {
            (true, true) => Field::Body,
            (false, false) if editing.body.read(cx).cursor_position().line == 0 => Field::Cue,
            _ => return false,
        };
        self.focus_field(to, window, cx);
        true
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
        self.go(Command::Goto(focus), cx);
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
            .map(|item| item.map(CardItem::title))
            .collect();
        // A spawned task runs outside any GPUI update, which the menu's own
        // event loop needs.
        cx.spawn_in(window, async move |this, cx| {
            let chosen =
                platform::context_menu(&ns_view, &titles, (f32::from(at.x), f32::from(at.y)));
            this.update_in(cx, |v, window, cx| {
                v.menu_target = None;
                match chosen.and_then(|i| CARD_MENU[i]) {
                    Some(CardItem::EditText) => v.edit_text_at(index, window, cx),
                    Some(CardItem::Op(op)) => v.edit_at(index, op, window, cx),
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

    /// A left press starts a move, or on an edge zone a resize. A double-click
    /// on an edge zone pushes that edge to the screen edge, and with Option,
    /// or from a centered strip, takes the whole width.
    fn press(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(frame) = platform::frame(window) else {
            return;
        };
        let screens = platform::screens();
        let Some(screen) = snap::holding(frame, &screens) else {
            return;
        };
        let Some(placement) = snap::classify(frame, &screens, Widths::default()) else {
            return;
        };
        let stick = placement.stick;
        let gesture = match snap::edge_at(f32::from(e.position.x), frame.w, stick) {
            None => Gesture::Move,
            Some(edge) if e.click_count == 2 => {
                let whole = e.modifiers.alt || stick == Stick::Stuck(Side::Center);
                let pushed = if whole {
                    snap::full_width(frame, screen)
                } else {
                    snap::pushed(frame, edge, screen)
                };
                self.place(pushed, window, cx);
                return;
            }
            Some(edge) => Gesture::Resize(edge),
        };
        let at = platform::pointer().at;
        self.drag = Some(Drag::start(gesture, frame, at, stick));
        // Polling, not GPUI mouse events, because `on_mouse_move` listeners
        // fire only while hovered and a fast drag leaves the window.
        self._drag_task = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(8))
                    .await;
                let live = this.update_in(cx, |this, window, cx| this.drag_tick(window, cx));
                if !live.unwrap_or(false) {
                    break;
                }
            }
        }));
    }

    /// One step of the gesture: the frame the pointer asks for, or on release
    /// the settled frame, which `place` saves once it lands. `false` once the
    /// gesture is over.
    fn drag_tick(&mut self, window: &Window, cx: &mut Context<Self>) -> bool {
        let Some(drag) = self.drag else {
            return false;
        };
        let p = platform::pointer();
        let screens = platform::screens();
        let frame = drag.step(p.at, p.option, &screens);
        if p.down {
            self.place(frame, window, cx);
            return true;
        }
        self.drag = None;
        self.place(snap::release(frame, &screens), window, cx);
        false
    }

    /// Bounds observer. A no-op while a drag lives, since the gesture owns the
    /// frame. Otherwise saves, which covers display changes and the restore.
    pub fn moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.drag.is_none() {
            self.persist(window, cx);
        }
    }

    /// Classifies the live frame, records the width, saves when changed, and
    /// says when the stick changed. A test window has no frame and keeps the
    /// saved placement. Nothing is saved mid-gesture or mid-move.
    fn persist(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.drag.is_some() || self.placing > 0 {
            return;
        }
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
            self.place(placed.on(target), window, cx);
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
            self.place(placement.stuck(side, frame, screen), window, cx);
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
        self.place(frame, window, cx);
    }

    /// Moves the strip to `frame` from a task, outside any update, because the
    /// move calls back into GPUI: AppKit reports the new bounds, and the
    /// redraw for the new frame renders on the spot. Saving waits until the
    /// move lands, so the frame macOS first shows the strip at, before the
    /// saved placement goes back on, is never saved.
    fn place(&mut self, frame: Rect, window: &Window, cx: &mut Context<Self>) {
        let Some(view) = platform::ns_view(window) else {
            return;
        };
        self.placing += 1;
        cx.spawn_in(window, async move |this, cx| {
            platform::move_and_redraw(&view, frame);
            this.update_in(cx, |this, window, cx| {
                this.placing -= 1;
                this.persist(window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Moves the strip, after saving any text edit.
    pub fn command(&mut self, cmd: Command, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(true, window, cx);
        self.go(cmd, cx);
    }

    fn go(&mut self, cmd: Command, cx: &mut Context<Self>) {
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

/// Where cards go on the strip and how big their text is.
struct Geometry {
    stage: Stage,
    stage_w: f32,
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

/// A card's place on the strip and its frame, before its content. Also the
/// size it is drawn at and its height, or `None` when it is off the strip or
/// invisible.
fn frame(g: &Geometry, look: &Appearance, t: &Theme) -> Option<(Div, f32, f32)> {
    let opacity = slot(look.d).opacity * look.opacity;
    let p = g.stage.place(look.d, look.shrink);
    let (w, h, size) = (p.w, p.h, p.size);
    if opacity <= 0.0 || p.left + w < 0.0 || p.left > g.stage_w {
        return None;
    }
    let frame = div()
        .absolute()
        .left(px(p.left))
        .top(px(p.top))
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
        .flex_col();
    Some((frame, size, h))
}

/// The line over a section's cards, with its label above it. `span` is the
/// ranks of the first and last card.
fn section_header(
    g: &Geometry,
    (key, span): (&RunKey, (f32, f32)),
    pos: f32,
    t: &Theme,
) -> Vec<AnyElement> {
    let pieces: Vec<_> = g
        .stage
        .header_pieces(span.0 - pos, span.1 - pos)
        .into_iter()
        .filter(|&(a, b)| b >= 0.0 && a <= g.stage_w)
        .collect();
    let y = g.stage.hang_top() - RAIL_GAP;
    let label = label_span(&pieces, (0.0, g.stage_w), HEADER_LABEL_MIN_W);
    let rules = pieces.into_iter().map(|(a, b)| {
        div()
            .absolute()
            .left(px(a))
            .top(px(y))
            .w(px(b - a))
            .h(px(1.0))
            .bg(t.dim.opacity(0.4))
            .into_any_element()
    });
    let label = label.map(|(left, right)| {
        div()
            .absolute()
            .left(px(left))
            .top(px(y - HEADER_LABEL_H))
            .w(px(right - left))
            .h(px(HEADER_LABEL_H))
            .line_height(px(HEADER_LABEL_H))
            .text_size(px(11.0))
            .text_color(t.dim)
            .truncate()
            .child(key.0.clone())
            .into_any_element()
    });
    rules.chain(label).collect()
}

/// A card at its slot, or `None` when it is off the strip or invisible.
fn card(
    g: &Geometry,
    layout: &CardLayout,
    look: Appearance,
    t: &Theme,
    window: &Window,
) -> Option<Div> {
    let (frame, size, _) = frame(g, &look, t)?;
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
    Some(frame.children(cue).children(body).children(more))
}

/// The card in the hot spot while its text is edited: the same frame, with
/// the cue and the body as fields in the sizes the rendered card uses. A body
/// taller than the card scrolls inside it.
fn editor(g: &Geometry, editing: &Editing, look: Appearance, t: &Theme) -> Option<Div> {
    let (frame, size, h) = frame(g, &look, t)?;
    let scale = g.scale;
    let cue_h = scale.cue * LINE_HEIGHT * size;
    let body_h = scale.body * LINE_HEIGHT * size;
    let body_area = h - 2.0 * (2.0 + CARD_PAD * size) - cue_h;
    // The fields inset their text by this much; the card's own text is not inset.
    let inset = px(4.0);
    let cue = Input::new(&editing.cue)
        .xsmall()
        .appearance(false)
        .bordered(false)
        .focus_bordered(false);
    let cue = Styled::h(cue, px(cue_h))
        .p_0()
        .text_size(px(scale.cue * size))
        .line_height(px(cue_h))
        .font_weight(FontWeight::BOLD)
        .text_color(t.fg);
    let body = Textarea::new(&editing.body)
        .xsmall()
        .appearance(false)
        .bordered(false)
        .h(px(body_area))
        .text_size(px(scale.body * size))
        .line_height(px(body_h))
        .text_color(t.fg);
    Some(
        frame
            .child(div().h(px(cue_h)).flex_none().child(cue))
            .child(div().h(px(body_area)).flex_none().mx(-inset).child(body)),
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
        let moving = self.motion.step(&ids, dt, slide);
        let moving = self.step_headers(dt, slide) || moving || self.pos != target;
        self.last_frame = moving.then_some(now);
        if moving {
            window.request_animation_frame();
        }

        let t = theme(window.appearance());
        let view = window.viewport_size();
        let (win_w, win_h) = (f32::from(view.width), f32::from(view.height));
        let stage_w = win_w - COLUMN_W;
        // Cards keep the width they had when a gesture began, so a live
        // resize doesn't lay out every card per frame; they reflow on release.
        let card_w = snap::card_width(self.drag.map_or(win_w, |d| d.grab.w));
        // The frame is read live, so the current card holds still on screen
        // while the window slides under it. Test windows have none.
        let screens = platform::screens();
        let placed = platform::frame(window).and_then(|f| {
            let screen = snap::holding(f, &screens)?;
            let stick = snap::classify(f, &screens, Widths::default())?.stick;
            Some((f, screen, stick))
        });
        let hot_x = placed.map_or(win_w / 2.0 - COLUMN_W, |(f, screen, _)| {
            snap::hot_spot(f, screen.frame, COLUMN_W, card_w)
        });
        // The resize zones, on the edges that can be grabbed. Each takes the
        // press itself, since a card under it would otherwise take it first.
        let zones: Vec<_> = placed.map_or_else(Vec::new, |(f, screen, stick)| {
            let symmetric = stick == Stick::Stuck(Side::Center);
            [Edge::Left, Edge::Right]
                .into_iter()
                .filter_map(|edge| {
                    let x = match edge {
                        Edge::Left => 0.0,
                        Edge::Right => f.w,
                    };
                    snap::edge_at(x, f.w, stick)?;
                    let cursor = match (edge, snap::lean(edge, f, screen, symmetric)) {
                        (_, Lean::Both) => CursorStyle::ResizeLeftRight,
                        (Edge::Left, Lean::Outward) | (Edge::Right, Lean::Inward) => {
                            CursorStyle::ResizeLeft
                        }
                        (Edge::Left, Lean::Inward) | (Edge::Right, Lean::Outward) => {
                            CursorStyle::ResizeRight
                        }
                    };
                    let zone = div()
                        .absolute()
                        .top_0()
                        .h_full()
                        .w(px(snap::EDGE_W))
                        .cursor(cursor)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, e: &MouseDownEvent, window, cx| {
                                cx.stop_propagation();
                                cx.activate(true);
                                this.press(e, window, cx);
                            }),
                        );
                    Some(match edge {
                        Edge::Left => zone.left_0(),
                        Edge::Right => zone.right_0(),
                    })
                })
                .collect()
        });
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
            stage: Stage {
                hot_x,
                card_w,
                card_h,
                win_h,
            },
            stage_w,
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
                let open = self.editing.as_ref().filter(|e| e.card == slide.id);
                let element = match open {
                    Some(editing) => editor(&geometry, editing, look, &t),
                    None => card(&geometry, layout, look, &t, window),
                }?
                .id(("slide", i))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                        if i != this.show.cursor() {
                            cx.stop_propagation();
                            cx.activate(true);
                            this.command(Command::Goto(i), window, cx);
                        } else if this.editing.is_some() {
                            cx.stop_propagation();
                        } else if e.click_count == 2 {
                            cx.stop_propagation();
                            this.begin_edit(window, cx);
                        }
                        // Otherwise the current card lets the press through to
                        // the strip, which drags the window.
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                        if this.editing.is_some() && i == this.show.cursor() {
                            cx.stop_propagation();
                        } else {
                            this.card_menu(i, e.position, window, cx)
                        }
                    }),
                );
                Some(element.into_any_element())
            });
        // Once a gesture has moved the strip, the cards are hidden and the
        // rest dimmed until release. macOS stalls the redraws that keep the
        // cards in step with the window often enough that they visibly
        // wobble. A click that hasn't moved it yet keeps them.
        let dragging = self
            .drag
            .zip(placed)
            .is_some_and(|(d, (f, _, _))| f != d.grab);
        let cards: Vec<_> = if dragging {
            Vec::new()
        } else {
            let headers = self
                .headers
                .iter()
                .flat_map(|(key, &span)| section_header(&geometry, (key, span), self.pos, &t))
                .collect::<Vec<_>>();
            headers.into_iter().chain(ghosts.chain(live)).collect()
        };

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
            .opacity(if dragging { 0.4 } else { 1.0 })
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
            .key_context(if self.editing.is_some() {
                app::EDITING
            } else {
                app::STRIP
            })
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &CancelEdit, window, cx| this.finish(false, window, cx)),
            )
            // Capture, because a multi-line field takes Enter for itself.
            .capture_action(cx.listener(|this, e: &text::Enter, window, cx| {
                if e.secondary && this.editing.is_some() {
                    cx.stop_propagation();
                    this.finish(true, window, cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &text::MoveDown, window, cx| {
                if this.cross(true, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &text::MoveUp, window, cx| {
                if this.cross(false, window, cx) {
                    cx.stop_propagation();
                }
            }))
            // The strip is a non-activating panel, so a click alone would leave the
            // previous app's menu bar showing.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e, window, cx| {
                    cx.activate(true);
                    this.press(e, window, cx);
                }),
            )
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
            .children(zones)
    }
}
