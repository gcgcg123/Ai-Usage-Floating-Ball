use serde::Serialize;

pub const COLLAPSED_LOGICAL_SIZE: f64 = 76.0;
pub const EXPANDED_LOGICAL_WIDTH: f64 = 312.0;
pub const EXPANDED_LOGICAL_HEIGHT: f64 = 270.0;
const BALL_LOGICAL_SIZE: f64 = 64.0;
const BALL_LOGICAL_MARGIN: f64 = 6.0;
const DOCK_SLIVER_LOGICAL_SIZE: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Placement {
    AboveLeft,
    AboveRight,
    BelowLeft,
    BelowRight,
}

impl Placement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AboveLeft => "above-left",
            Self::AboveRight => "above-right",
            Self::BelowLeft => "below-left",
            Self::BelowRight => "below-right",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dock {
    #[default]
    None,
    Left,
    Right,
    Top,
}

impl Dock {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    fn right(self) -> f64 {
        self.x + self.width
    }

    fn bottom(self) -> f64 {
        self.y + self.height
    }
}

fn edge_offset(scale: f64) -> f64 {
    (BALL_LOGICAL_MARGIN + BALL_LOGICAL_SIZE / 2.0) * scale
}

pub fn collapsed_size(scale: f64) -> Size {
    Size {
        width: COLLAPSED_LOGICAL_SIZE * scale,
        height: COLLAPSED_LOGICAL_SIZE * scale,
    }
}

pub fn expanded_size(scale: f64) -> Size {
    Size {
        width: EXPANDED_LOGICAL_WIDTH * scale,
        height: EXPANDED_LOGICAL_HEIGHT * scale,
    }
}

pub fn ball_center(position: Point, size: Size, placement: Placement, scale: f64) -> Point {
    let offset = edge_offset(scale);
    let x = match placement {
        Placement::AboveLeft | Placement::BelowLeft => position.x + size.width - offset,
        Placement::AboveRight | Placement::BelowRight => position.x + offset,
    };
    let y = match placement {
        Placement::AboveLeft | Placement::AboveRight => position.y + size.height - offset,
        Placement::BelowLeft | Placement::BelowRight => position.y + offset,
    };

    Point::new(x, y)
}

pub fn choose_placement(anchor: Point, monitor: Rect, scale: f64) -> Placement {
    let expanded = expanded_size(scale);
    let offset = edge_offset(scale);
    let required_left = expanded.width - offset;
    let required_right = expanded.width - offset;
    let required_above = expanded.height - offset;
    let required_below = expanded.height - offset;

    let space_left = anchor.x - monitor.x;
    let space_right = monitor.right() - anchor.x;
    let space_above = anchor.y - monitor.y;
    let space_below = monitor.bottom() - anchor.y;

    let place_left = if space_left >= required_left {
        true
    } else if space_right >= required_right {
        false
    } else {
        space_left >= space_right
    };

    let place_above = if space_above >= required_above {
        true
    } else if space_below >= required_below {
        false
    } else {
        space_above >= space_below
    };

    match (place_above, place_left) {
        (true, true) => Placement::AboveLeft,
        (true, false) => Placement::AboveRight,
        (false, true) => Placement::BelowLeft,
        (false, false) => Placement::BelowRight,
    }
}

pub fn expanded_position(anchor: Point, placement: Placement, monitor: Rect, scale: f64) -> Point {
    let size = expanded_size(scale);
    let offset = edge_offset(scale);
    let x = match placement {
        Placement::AboveLeft | Placement::BelowLeft => anchor.x - (size.width - offset),
        Placement::AboveRight | Placement::BelowRight => anchor.x - offset,
    };
    let y = match placement {
        Placement::AboveLeft | Placement::AboveRight => anchor.y - (size.height - offset),
        Placement::BelowLeft | Placement::BelowRight => anchor.y - offset,
    };

    clamp_position(Point::new(x, y), size, monitor)
}

pub fn collapsed_position(anchor: Point, scale: f64) -> Point {
    let size = collapsed_size(scale);
    Point::new(anchor.x - size.width / 2.0, anchor.y - size.height / 2.0)
}

pub fn docked_collapsed_position(center: Point, monitor: Rect, scale: f64, dock: Dock) -> Point {
    if dock == Dock::None {
        return collapsed_position(center, scale);
    }

    // Distance from the screen edge to the ball center when 12px of the ball
    // stays visible: 32px (half ball) - 12px (sliver).
    let hidden = (BALL_LOGICAL_SIZE / 2.0 - DOCK_SLIVER_LOGICAL_SIZE) * scale;
    let ball_center_offset = (BALL_LOGICAL_MARGIN + BALL_LOGICAL_SIZE / 2.0) * scale;
    let center_x = center.x.clamp(
        monitor.x + ball_center_offset,
        (monitor.right() - ball_center_offset).max(monitor.x + ball_center_offset),
    );
    let center_y = center.y.clamp(
        monitor.y + ball_center_offset,
        (monitor.bottom() - ball_center_offset - 52.0 * scale).max(monitor.y + ball_center_offset),
    );

    match dock {
        Dock::Left => Point::new(monitor.x - hidden - ball_center_offset, center_y),
        Dock::Right => Point::new(monitor.right() + hidden - ball_center_offset, center_y),
        Dock::Top => Point::new(center_x, monitor.y - hidden - ball_center_offset),
        Dock::None => unreachable!("handled above"),
    }
}

fn clamp_position(position: Point, size: Size, monitor: Rect) -> Point {
    let min_x = monitor.x;
    let min_y = monitor.y;
    let max_x = (monitor.right() - size.width).max(min_x);
    let max_y = (monitor.bottom() - size.height).max(min_y);

    Point::new(
        position.x.clamp(min_x, max_x),
        position.y.clamp(min_y, max_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCALE: f64 = 1.25;
    const MONITOR: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1536.0,
        height: 864.0,
    };

    #[test]
    fn chooses_above_left_for_anchor_near_bottom_right() {
        let placement = choose_placement(Point::new(1450.0, 780.0), MONITOR, SCALE);

        assert_eq!(placement, Placement::AboveLeft);
    }

    #[test]
    fn chooses_below_right_for_anchor_near_top_left() {
        let placement = choose_placement(Point::new(60.0, 60.0), MONITOR, SCALE);

        assert_eq!(placement, Placement::BelowRight);
    }

    #[test]
    fn expanded_position_preserves_ball_center() {
        let anchor = Point::new(1450.0, 780.0);
        let placement = Placement::AboveLeft;
        let position = expanded_position(anchor, placement, MONITOR, SCALE);
        let center = ball_center(position, expanded_size(SCALE), placement, SCALE);

        assert!((center.x - anchor.x).abs() < 0.01);
        assert!((center.y - anchor.y).abs() < 0.01);
    }

    #[test]
    fn collapsed_position_preserves_ball_center() {
        let anchor = Point::new(500.0, 300.0);
        let position = collapsed_position(anchor, SCALE);
        let center = Point::new(
            position.x + collapsed_size(SCALE).width / 2.0,
            position.y + collapsed_size(SCALE).height / 2.0,
        );

        assert!((center.x - anchor.x).abs() < 0.01);
        assert!((center.y - anchor.y).abs() < 0.01);
    }

    #[test]
    fn docked_positions_leave_only_a_sliver_visible() {
        let anchor = Point::new(500.0, 400.0);

        let left = docked_collapsed_position(anchor, MONITOR, SCALE, Dock::Left);
        assert!((left.x + collapsed_size(SCALE).width - MONITOR.x - 18.0 * SCALE).abs() < 0.01);

        let right = docked_collapsed_position(anchor, MONITOR, SCALE, Dock::Right);
        assert!((MONITOR.right() - right.x - 18.0 * SCALE).abs() < 0.01);

        let top = docked_collapsed_position(anchor, MONITOR, SCALE, Dock::Top);
        assert!((top.y + collapsed_size(SCALE).height - MONITOR.y - 18.0 * SCALE).abs() < 0.01);
    }

    #[test]
    fn dock_from_str_round_trips() {
        assert_eq!(Dock::from_str("left"), Some(Dock::Left));
        assert_eq!(Dock::from_str("RIGHT"), Some(Dock::Right));
        assert_eq!(Dock::from_str("top").unwrap().as_str(), "top");
        assert_eq!(Dock::from_str("diagonal"), None);
    }
}
