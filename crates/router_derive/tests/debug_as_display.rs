use router_derive::DebugAsDisplay;

#[derive(Debug, DebugAsDisplay)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, DebugAsDisplay)]
enum Direction {
    Up,
}

#[derive(Debug, DebugAsDisplay)]
struct Wrapper<T: std::fmt::Debug>(T);

#[test]
fn test_display_matches_debug() {
    let point = Point { x: 1, y: -2 };

    assert_eq!(point.to_string(), format!("{point:?}"));
    assert_eq!(point.to_string(), "Point { x: 1, y: -2 }");
    assert_eq!(Direction::Up.to_string(), "Up");
    assert_eq!(Wrapper(point).to_string(), "Wrapper(Point { x: 1, y: -2 })");
}

#[test]
fn test_display_ignores_formatter_flags() {
    let point = Point { x: 1, y: -2 };

    // Flags passed to `Display` are not forwarded to the underlying `Debug` implementation
    assert_eq!(format!("{point:#}"), "Point { x: 1, y: -2 }");
    assert_eq!(format!("{:>8}", Direction::Up), "Up");
}
