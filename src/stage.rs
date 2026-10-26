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
    /// The card at distance `d` from the hot spot, scaled further by `shrink`.
    pub fn place(&self, d: f32, shrink: f32) -> Placement {
        let s = slot(d);
        let size = s.scale * shrink;
        let (w, h) = (self.card_w * size, self.card_h * size);
        Placement {
            left: self.hot_x + s.x * self.card_w - w / 2.0,
            top: (self.win_h - h) / 2.0,
            w,
            h,
            size,
        }
    }
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
