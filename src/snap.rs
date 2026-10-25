//! Where the strip sits on the screens: sticks, magnets, resize, and the hot
//! spot. Every function is pure, in AppKit space (points, y up). The shell in
//! platform.rs reads frames and screens in, writes one frame out, and keeps no
//! placement state of its own.

use schemars::JsonSchema;
use serde::Deserialize;

pub const MIN_W: f32 = 600.0;
pub const DEFAULT_H: f32 = 150.0;
/// A magnet catches an edge within this reach on the open side of a screen.
pub const CATCH: f32 = 24.0;
/// Past a screen or display edge, a magnet holds on until pushed this far.
pub const HOLD: f32 = 48.0;
/// The resize hit zone on each outer edge of the strip.
pub const EDGE_W: f32 = 8.0;
const MAX_CARD_W: f32 = 420.0;
/// The width a Left or Right stick takes when none was remembered.
const SIDE_W: f32 = 1200.0;
/// An edge this close to a line counts as on it.
const FLUSH: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn top(&self) -> f32 {
        self.y + self.h
    }

    fn center_x(&self) -> f32 {
        self.x + self.w / 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// One display. `frame` is the whole screen, `visible` excludes the menu bar
/// and Dock.
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    pub name: String,
    pub frame: Rect,
    pub visible: Rect,
}

/// The three places a strip can be stuck. Also the width-memory key and the
/// menu action payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum Side {
    Left,
    Center,
    Right,
}

/// Derived from the frame, never set. Only `Free` owns an x, so a saved stick
/// and a saved x cannot disagree. `x` and `w` are screen-local points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stick {
    Stuck(Side),
    Free { x: f32, w: f32 },
}

impl Stick {
    pub fn side(self) -> Option<Side> {
        match self {
            Stick::Stuck(side) => Some(side),
            Stick::Free { .. } => None,
        }
    }
}

/// The width the strip last had on each side. `classify` records the current
/// side's width on every persist, so "the width when left" is simply the last
/// width recorded there.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Widths([Option<f32>; 3]);

impl Widths {
    pub fn get(&self, side: Side) -> Option<f32> {
        self.0[side as usize]
    }

    pub(crate) fn with(mut self, side: Side, w: f32) -> Self {
        self.0[side as usize] = Some(w);
        self
    }
}

/// What survives a relaunch. `top` is screen-local, y down, 0 = flush with the
/// very top of the screen, over the menu bar. `display` names the screen;
/// `None` or an unknown name means the main screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub stick: Stick,
    pub top: f32,
    pub h: f32,
    pub display: Option<String>,
    pub widths: Widths,
}

impl Placement {
    /// The frame this placement means on the connected screens. An unknown
    /// display falls back to the main screen. `None` with no screens.
    pub fn frame_on(&self, screens: &[Screen]) -> Option<Rect> {
        let named = self
            .display
            .as_deref()
            .and_then(|name| screens.iter().find(|s| s.name == name));
        Some(self.on(named.or(screens.first())?))
    }

    /// The frame this placement means on `screen`. A stuck side derives x and
    /// w from the side and the width remembered for it (default: the screen
    /// width for Center, 1200 for Left and Right). Free keeps its x and w.
    /// Everything is clamped into the screen.
    pub fn on(&self, screen: &Screen) -> Rect {
        let s = screen.frame;
        let (x, w) = match self.stick {
            Stick::Stuck(side) => {
                let remembered = self.widths.get(side).unwrap_or(match side {
                    Side::Center => s.w,
                    Side::Left | Side::Right => SIDE_W,
                });
                let w = fit_w(remembered, s.w);
                (side_x(side, w, s), w)
            }
            Stick::Free { x, w } => {
                let w = fit_w(w, s.w);
                (s.x + x.min(s.w - w).max(0.0), w)
            }
        };
        let h = self.h.min(s.h);
        let top = self.top.min(s.h - h).max(0.0);
        Rect {
            x,
            y: s.top() - top - h,
            w,
            h,
        }
    }

    /// The frame a menu command asks for: `side` on `screen`, at the width
    /// remembered for that side, else the current width.
    pub fn stuck(&self, side: Side, current: Rect, screen: &Screen) -> Rect {
        let s = screen.frame;
        let w = fit_w(self.widths.get(side).unwrap_or(current.w), s.w);
        Rect {
            x: side_x(side, w, s),
            w,
            ..current
        }
    }
}

/// `w` kept between the floor and the screen width. A screen narrower than
/// the floor floors at its own width.
fn fit_w(w: f32, screen_w: f32) -> f32 {
    w.clamp(MIN_W.min(screen_w), screen_w)
}

fn side_x(side: Side, w: f32, s: Rect) -> f32 {
    match side {
        Side::Left => s.x,
        Side::Center => s.x + (s.w - w) / 2.0,
        Side::Right => s.right() - w,
    }
}

/// Reads the placement off a live frame: both edges flush or centered is
/// Center, then flush left, then flush right, else Free. `widths` is carried
/// over with the stuck side's width recorded. `None` with no screens.
pub fn classify(frame: Rect, screens: &[Screen], widths: Widths) -> Option<Placement> {
    let screen = holding(frame, screens)?;
    let s = screen.frame;
    let x = frame.x - s.x;
    let flush = |d: f32| d.abs() <= FLUSH;
    let left = flush(x);
    let right = flush(x + frame.w - s.w);
    let centered = flush(x + frame.w / 2.0 - s.w / 2.0);
    let stick = if left && right || centered {
        Stick::Stuck(Side::Center)
    } else if left {
        Stick::Stuck(Side::Left)
    } else if right {
        Stick::Stuck(Side::Right)
    } else {
        Stick::Free { x, w: frame.w }
    };
    Some(Placement {
        stick,
        top: s.top() - frame.top(),
        h: frame.h,
        display: Some(screen.name.clone()),
        widths: match stick {
            Stick::Stuck(side) => widths.with(side, frame.w),
            Stick::Free { .. } => widths,
        },
    })
}

/// The screen holding the largest part of `frame`, else the first screen.
pub fn holding(frame: Rect, screens: &[Screen]) -> Option<&Screen> {
    let mut best: Option<(&Screen, f32)> = None;
    for screen in screens {
        let area = overlap(frame, screen.frame);
        if area > 0.0 && best.is_none_or(|(_, most)| area > most) {
            best = Some((screen, area));
        }
    }
    best.map(|(screen, _)| screen).or(screens.first())
}

fn overlap(a: Rect, b: Rect) -> f32 {
    let w = (a.right().min(b.right()) - a.x.max(b.x)).max(0.0);
    let h = (a.top().min(b.top()) - a.y.max(b.y)).max(0.0);
    w * h
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Move,
    Resize(Edge),
}

/// A gesture in flight. Everything is anchored at the grab, so each step is a
/// pure function of the pointer now. No frame from a previous step feeds the
/// next one, which is what lets a magnet resist and then let go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub gesture: Gesture,
    pub grab: Rect,
    pub at: Point,
    /// Both edges move at once. Starts true for a strip stuck Center, and
    /// Option makes it true per step.
    pub symmetric: bool,
}

impl Drag {
    pub fn start(gesture: Gesture, grab: Rect, at: Point, stick: Stick) -> Self {
        Self {
            gesture,
            grab,
            at,
            symmetric: stick == Stick::Stuck(Side::Center),
        }
    }

    /// The frame the pointer asks for before any magnet. Move shifts the grab
    /// frame. Resize moves the grabbed edge by the pointer's dx, and the other
    /// edge by -dx when symmetric.
    pub fn target(&self, pointer: Point, option: bool) -> Rect {
        let (dx, dy) = (pointer.x - self.at.x, pointer.y - self.at.y);
        let g = self.grab;
        match self.gesture {
            Gesture::Move => Rect {
                x: g.x + dx,
                y: g.y + dy,
                ..g
            },
            Gesture::Resize(edge) => {
                let (left, right) = match (edge, self.symmetric || option) {
                    (Edge::Left, false) => (g.x + dx, g.right()),
                    (Edge::Left, true) => (g.x + dx, g.right() - dx),
                    (Edge::Right, false) => (g.x, g.right() + dx),
                    (Edge::Right, true) => (g.x - dx, g.right() + dx),
                };
                Rect {
                    x: left,
                    w: right - left,
                    ..g
                }
            }
        }
    }

    /// `target` after magnets and clamps. Move: each axis takes the magnet
    /// holding it, else the target. Resize: magnets act on the dragged edge
    /// only, on the screen holding the grab, then the width is clamped between
    /// the floor and that screen.
    pub fn step(&self, pointer: Point, option: bool, screens: &[Screen]) -> Rect {
        let t = self.target(pointer, option);
        let Gesture::Resize(edge) = self.gesture else {
            return Rect {
                x: pull(t.x, move_x_magnets(t.w, screens)),
                y: pull(t.y, move_y_magnets(t.h, screens)),
                ..t
            };
        };
        let Some(screen) = holding(self.grab, screens) else {
            return t;
        };
        let s = screen.frame;
        let symmetric = self.symmetric || option;
        let g = self.grab;
        let center = g.center_x();
        let pulled = match edge {
            Edge::Left => pull(t.x, edge_magnets(s)),
            Edge::Right => pull(t.right(), edge_magnets(s)),
        };
        let w = match (edge, symmetric) {
            (Edge::Left, false) => g.right() - pulled,
            (Edge::Right, false) => pulled - g.x,
            (Edge::Left, true) => 2.0 * (center - pulled),
            (Edge::Right, true) => 2.0 * (pulled - center),
        };
        let max_w = match (edge, symmetric) {
            (Edge::Left, false) => g.right() - s.x,
            (Edge::Right, false) => s.right() - g.x,
            (_, true) => 2.0 * (center - s.x).min(s.right() - center),
        };
        let max_w = max_w.max(MIN_W.min(s.w));
        let w = w.clamp(MIN_W.min(max_w), max_w);
        let x = match (edge, symmetric) {
            (Edge::Left, false) => g.right() - w,
            (Edge::Right, false) => g.x,
            (_, true) => center - w / 2.0,
        };
        Rect { x, w, ..t }
    }
}

/// A line a position is pulled to while it is within `below` under the line
/// or `above` over it.
struct Magnet {
    at: f32,
    below: f32,
    above: f32,
}

/// `pos` pulled to the magnet holding it, the nearest in units of its reach
/// when several do, else `pos`.
fn pull(pos: f32, magnets: impl IntoIterator<Item = Magnet>) -> f32 {
    magnets
        .into_iter()
        .filter_map(|m| {
            let d = pos - m.at;
            let reach = if d < 0.0 { m.below } else { m.above };
            (d.abs() <= reach).then_some((m.at, d.abs() / reach))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(pos, |(at, _)| at)
}

/// The x origins a moving strip of width `w` is pulled to: flush with every
/// screen's left and right edge, caught from inside the screen and held
/// against leaving it, and centered on every screen. Adjacent screens share
/// edges, so a shared display edge resists from both sides.
fn move_x_magnets(w: f32, screens: &[Screen]) -> impl Iterator<Item = Magnet> + '_ {
    screens.iter().flat_map(move |screen| {
        let s = screen.frame;
        [
            Magnet {
                at: s.x,
                below: HOLD,
                above: CATCH,
            },
            Magnet {
                at: s.right() - w,
                below: CATCH,
                above: HOLD,
            },
            Magnet {
                at: s.center_x() - w / 2.0,
                below: CATCH,
                above: CATCH,
            },
        ]
    })
}

/// The y origins a moving strip of height `h` is pulled to: its top on every
/// screen's top edge, held against going over it, and under every menu bar.
fn move_y_magnets(h: f32, screens: &[Screen]) -> impl Iterator<Item = Magnet> + '_ {
    screens.iter().flat_map(move |screen| {
        [
            Magnet {
                at: screen.frame.top() - h,
                below: CATCH,
                above: HOLD,
            },
            Magnet {
                at: screen.visible.top() - h,
                below: CATCH,
                above: CATCH,
            },
        ]
    })
}

/// Where a resized edge is pulled: the screen's edges and its center line.
fn edge_magnets(s: Rect) -> [Magnet; 3] {
    [
        Magnet {
            at: s.x,
            below: HOLD,
            above: CATCH,
        },
        Magnet {
            at: s.right(),
            below: CATCH,
            above: HOLD,
        },
        Magnet {
            at: s.center_x(),
            below: CATCH,
            above: CATCH,
        },
    ]
}

/// After a release: a frame straddling screens lands wholly on the one
/// holding most of it, narrowed to it if wider, and a frame off every screen
/// lands on the main one. A frame on a single screen, hanging off its outer
/// edge or not, stays put.
pub fn release(frame: Rect, screens: &[Screen]) -> Rect {
    let on = screens
        .iter()
        .filter(|s| overlap(frame, s.frame) > 0.0)
        .count();
    match holding(frame, screens) {
        Some(screen) if on != 1 => inside(frame, screen.frame),
        _ => frame,
    }
}

fn inside(frame: Rect, s: Rect) -> Rect {
    let w = frame.w.min(s.w);
    let h = frame.h.min(s.h);
    Rect {
        x: frame.x.min(s.right() - w).max(s.x),
        y: frame.y.min(s.top() - h).max(s.y),
        w,
        h,
    }
}

/// Which edge a press at window-local `x` grabs. The stuck edge of a Left or
/// Right strip is not grabbable. Both edges of a Center strip are, full width
/// included.
pub fn edge_at(x: f32, w: f32, stick: Stick) -> Option<Edge> {
    let edge = if x < EDGE_W {
        Edge::Left
    } else if x > w - EDGE_W {
        Edge::Right
    } else {
        return None;
    };
    let stuck = match edge {
        Edge::Left => Side::Left,
        Edge::Right => Side::Right,
    };
    (stick != Stick::Stuck(stuck)).then_some(edge)
}

/// Which way an edge can still move, for the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lean {
    Both,
    Inward,
    Outward,
}

/// At the screen edge only inward, at the floor only outward. A symmetric
/// resize is at the screen edge when either edge is.
pub fn lean(edge: Edge, frame: Rect, screen: &Screen, symmetric: bool) -> Lean {
    let s = screen.frame;
    let at_left = frame.x <= s.x + FLUSH;
    let at_right = frame.right() >= s.right() - FLUSH;
    let at_max = match (edge, symmetric) {
        (_, true) => at_left || at_right,
        (Edge::Left, false) => at_left,
        (Edge::Right, false) => at_right,
    };
    if at_max {
        Lean::Inward
    } else if frame.w <= MIN_W.min(s.w) + FLUSH {
        Lean::Outward
    } else {
        Lean::Both
    }
}

/// Double-click on an edge: that edge to the screen edge.
pub fn pushed(frame: Rect, edge: Edge, screen: &Screen) -> Rect {
    let s = screen.frame;
    let floor = MIN_W.min(s.w);
    match edge {
        Edge::Left => {
            let w = (frame.right() - s.x).max(floor);
            Rect {
                x: frame.right() - w,
                w,
                ..frame
            }
        }
        Edge::Right => Rect {
            w: (s.right() - frame.x).max(floor),
            ..frame
        },
    }
}

/// Option-double-click on an edge: the whole screen width.
pub fn full_width(frame: Rect, screen: &Screen) -> Rect {
    Rect {
        x: screen.frame.x,
        w: screen.frame.w,
        ..frame
    }
}

/// Stage-local x of the hot spot: the screen's center, clamped so the current
/// card stays whole on the stage, right of the column.
pub fn hot_spot(frame: Rect, screen: Rect, column_w: f32, card_w: f32) -> f32 {
    let stage_w = frame.w - column_w;
    let center = screen.center_x() - frame.x - column_w;
    center.min(stage_w - card_w / 2.0).max(card_w / 2.0)
}

/// Card width for a strip laid out `layout_w` wide.
pub fn card_width(layout_w: f32) -> f32 {
    MAX_CARD_W.min(0.3 * layout_w)
}
