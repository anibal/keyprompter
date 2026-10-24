/// A rectangle in any one coordinate space, y up or y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The origin that lands each window edge on a nearby screen edge, or on the
/// edge of the visible area that excludes the menu bar and Dock. An axis with
/// no edge within `reach` keeps its position. `None` when nothing moves.
pub fn settle(win: Rect, screen: Rect, visible: Rect, reach: f32) -> Option<(f32, f32)> {
    let x = nearest(
        win.x,
        win.w,
        [screen.x, visible.x],
        [screen.x + screen.w, visible.x + visible.w],
        reach,
    );
    let y = nearest(
        win.y,
        win.h,
        [screen.y, visible.y],
        [screen.y + screen.h, visible.y + visible.h],
        reach,
    );
    (x != win.x || y != win.y).then_some((x, y))
}

fn nearest(pos: f32, len: f32, starts: [f32; 2], ends: [f32; 2], reach: f32) -> f32 {
    starts
        .into_iter()
        .chain(ends.into_iter().map(|end| end - len))
        .map(|candidate| (candidate, (candidate - pos).abs()))
        .filter(|&(_, distance)| distance <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(pos, |(candidate, _)| candidate)
}

/// `win` centered on the top edge of `screen` and no wider than it, in a
/// y-up space such as AppKit's.
pub fn top_center(win: Rect, screen: Rect) -> Rect {
    let w = win.w.min(screen.w);
    Rect {
        x: screen.x + (screen.w - w) / 2.0,
        y: screen.y + screen.h - win.h,
        w,
        h: win.h,
    }
}

pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}
