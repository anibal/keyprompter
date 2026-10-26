/// Placement of one slide, as a pure function of its distance `d` from the
/// hot spot (`slide index - animated position`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// Card center relative to the hot-spot center, in full-size card widths.
    pub x: f32,
    pub scale: f32,
    pub opacity: f32,
}

const TIME_CONSTANT: f32 = 0.09;
const SNAP: f32 = 0.001;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub fn slot(d: f32) -> Slot {
    if d >= 0.0 {
        Slot {
            x: lerp(0.0, 0.95, d) + 0.85 * (d - 1.0).max(0.0),
            scale: lerp(1.0, 0.8, d),
            opacity: lerp(1.0, 0.85, d) * lerp(1.0, 0.0, d - 2.5),
        }
    } else {
        let a = -d;
        Slot {
            x: -(lerp(0.0, 0.85, a) + 0.65 * (a - 1.0).max(0.0)),
            scale: lerp(1.0, 0.6, a),
            opacity: lerp(1.0, 0.55, a) * lerp(1.0, 0.0, a - 2.5),
        }
    }
}

/// The strip's measurements, for placing a card at a distance from the hot spot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage {
    /// Center of the hot spot, from the stage's left edge.
    pub hot_x: f32,
    /// Width of a full-size card.
    pub card_w: f32,
    pub card_h: f32,
    pub win_h: f32,
}

/// Where a card sits on the stage and how large it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub left: f32,
    pub top: f32,
    pub w: f32,
    pub h: f32,
    /// The card's size relative to a full-size one.
    pub size: f32,
}

impl Stage {
    /// Top of the card in the hot spot.
    pub fn mid_top(&self) -> f32 {
        (self.win_h - self.card_h) / 2.0
    }

    /// The top every side card hangs from: low enough that an upcoming
    /// card's bottom meets the middle card's.
    pub fn hang_top(&self) -> f32 {
        self.mid_top() + self.card_h * (1.0 - slot(1.0).scale)
    }

    /// The card at distance `d` from the hot spot, scaled further by `shrink`.
    pub fn place(&self, d: f32, shrink: f32) -> Placement {
        let s = slot(d);
        let size = s.scale * shrink;
        let (w, h) = (self.card_w * size, self.card_h * size);
        Placement {
            left: self.hot_x + s.x * self.card_w - w / 2.0,
            top: lerp(self.mid_top(), self.hang_top(), d.abs()),
            w,
            h,
            size,
        }
    }
}

impl Stage {
    /// The pieces of a section's line for a run of cards at distances
    /// `first_d..=last_d`, left to right. Each ends at a side card's edge, so
    /// the middle card belongs to neither, and stays continuous as the strip
    /// slides. A piece with no width is dropped.
    pub fn header_pieces(&self, first_d: f32, last_d: f32) -> Vec<(f32, f32)> {
        let (first, last) = (self.place(first_d, 1.0), self.place(last_d, 1.0));
        let (prev, next) = (self.place(-1.0, 1.0), self.place(1.0, 1.0));
        let (start, end) = (first.left, last.left + last.w);
        [
            (start, end.min(prev.left + prev.w)),
            (start.max(next.left), end),
        ]
        .into_iter()
        .filter(|(a, b)| b > a)
        .collect()
    }
}

/// Where a section's label goes: the left end of the first of `pieces`, left
/// to right, that is at least `min_w` wide once clamped to the `visible`
/// stage. `None` when no piece is.
pub fn label_span(pieces: &[(f32, f32)], visible: (f32, f32), min_w: f32) -> Option<(f32, f32)> {
    pieces
        .iter()
        .map(|&(a, b)| (a.max(visible.0), b.min(visible.1)))
        .find(|(a, b)| b - a >= min_w)
}

/// Exponential smoothing toward `target`, stable under any frame time.
pub fn ease(pos: f32, target: f32, dt_secs: f32) -> f32 {
    if (target - pos).abs() < SNAP {
        return target;
    }
    let next = pos + (target - pos) * (1.0 - (-dt_secs / TIME_CONSTANT).exp());
    if (target - next).abs() < SNAP {
        target
    } else {
        next
    }
}
