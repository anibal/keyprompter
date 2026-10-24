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
