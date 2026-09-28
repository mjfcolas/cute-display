use embedded_graphics::prelude::{Point, Size};
use embedded_graphics::primitives::Rectangle;

pub(crate) struct Scope {
    pub center: Point,
    pub radius_px: i32,
    pub range_km: f64,
}

impl Scope {
    pub fn locate(&self, east_km: f64, north_km: f64) -> Option<Point> {
        if self.range_km <= 0.0 || east_km.hypot(north_km) > self.range_km {
            return None;
        }
        let scale = f64::from(self.radius_px) / self.range_km;
        Some(self.center + Point::new((east_km * scale).round() as i32, -(north_km * scale).round() as i32))
    }
}

pub(crate) fn aircraft_triangle(at: Point, track_degrees: f32, length: i32) -> [Point; 3] {
    let heading = f64::from(track_degrees).to_radians();
    let (sin, cos) = heading.sin_cos();
    let point = |forward: f64, right: f64| {
        let east = forward * sin + right * cos;
        let north = forward * cos - right * sin;
        at + Point::new(east.round() as i32, -north.round() as i32)
    };
    let half = f64::from(length) / 2.0;
    [point(half, 0.0), point(-half, half * 0.8), point(-half, -half * 0.8)]
}

/// Where a label may sit around what it names, in the order they are tried: beside it
/// first, since that reads best, then above and below, then the corners.
const LABEL_SIDES: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, -1), (0, 1), (1, -1), (-1, -1), (1, 1), (-1, 1)];

pub(crate) struct LabelWanted {
    pub near: Point,
    pub size: Size,
}

pub(crate) fn place_labels(wanted: &[LabelWanted], obstacles: &[Rectangle], scope: &Scope, clearance: i32) -> Vec<Option<Rectangle>> {
    let mut taken: Vec<Rectangle> = obstacles.to_vec();
    wanted
        .iter()
        .map(|label| {
            let (w, h) = (label.size.width as i32, label.size.height as i32);
            let placed = LABEL_SIDES.iter().map(|&(dx, dy)| {
                let x = match dx {
                    1 => label.near.x + clearance,
                    -1 => label.near.x - clearance - w,
                    _ => label.near.x - w / 2,
                };
                let y = match dy {
                    1 => label.near.y + clearance,
                    -1 => label.near.y - clearance - h,
                    _ => label.near.y - h / 2,
                };
                Rectangle::new(Point::new(x, y), label.size)
            });
            let free = placed.into_iter().find(|candidate| {
                scope.contains(candidate) && taken.iter().all(|other| candidate.intersection(other).is_zero_sized())
            });
            if let Some(rectangle) = free {
                taken.push(rectangle);
            }
            free
        })
        .collect()
}

impl Scope {
    fn contains(&self, rectangle: &Rectangle) -> bool {
        let Some(bottom_right) = rectangle.bottom_right() else { return false };
        [rectangle.top_left, bottom_right, Point::new(rectangle.top_left.x, bottom_right.y), Point::new(bottom_right.x, rectangle.top_left.y)]
            .iter()
            .all(|corner| {
                let d = *corner - self.center;
                d.x * d.x + d.y * d.y <= self.radius_px * self.radius_px
            })
    }
}

/// What an aircraft is called on the scope: the last two letters of its registration,
/// `F-GKXA` being `XA`, which is how the airframe is known on the radio.
pub(crate) fn short_registration(registration: &str) -> Option<String> {
    let letters: Vec<char> = registration.chars().filter(char::is_ascii_alphanumeric).collect();
    let tail = letters.get(letters.len().checked_sub(2)?..)?;
    Some(tail.iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Scope {
        Scope { center: Point::new(100, 100), radius_px: 80, range_km: 25.0 }
    }

    #[test]
    fn north_east_is_up_and_to_the_right() {
        let at = scope().locate(10.0, 10.0).unwrap();
        assert!(at.x > 100 && at.y < 100, "{at:?}");
    }

    #[test]
    fn the_edge_of_the_range_is_the_edge_of_the_scope() {
        assert_eq!(scope().locate(25.0, 0.0), Some(Point::new(180, 100)));
        assert_eq!(scope().locate(0.0, -25.0), Some(Point::new(100, 180)));
        assert_eq!(scope().locate(20.0, 20.0), None, "28 km out is beyond 25");
    }

    #[test]
    fn a_triangle_points_where_the_aircraft_heads() {
        let center = Point::new(50, 50);
        let [nose, ..] = aircraft_triangle(center, 0.0, 10);
        assert_eq!(nose, Point::new(50, 45), "north is up");
        let [nose, ..] = aircraft_triangle(center, 90.0, 10);
        assert_eq!(nose, Point::new(55, 50), "east is right");
        let [nose, ..] = aircraft_triangle(center, 180.0, 10);
        assert_eq!(nose, Point::new(50, 55), "south is down");
    }

    fn wanted(x: i32, y: i32) -> LabelWanted {
        LabelWanted { near: Point::new(x, y), size: Size::new(12, 10) }
    }

    fn overlap(a: &Rectangle, b: &Rectangle) -> bool {
        !a.intersection(b).is_zero_sized()
    }

    #[test]
    fn a_label_goes_beside_what_it_names_when_there_is_room() {
        let placed = place_labels(&[wanted(100, 100)], &[], &scope(), 5);
        assert_eq!(placed, [Some(Rectangle::new(Point::new(105, 95), Size::new(12, 10)))]);
    }

    #[test]
    fn labels_on_a_crowded_spot_take_other_sides_and_never_overlap() {
        let crowd: Vec<LabelWanted> = (0..6).map(|n| wanted(100 + n, 100)).collect();
        let mark = Rectangle::new(Point::new(96, 96), Size::new(12, 9));
        let placed = place_labels(&crowd, &[mark], &scope(), 5);
        let rectangles: Vec<Rectangle> = placed.iter().flatten().copied().collect();
        assert!(rectangles.len() >= 4, "only {} placed", rectangles.len());
        for (n, a) in rectangles.iter().enumerate() {
            assert!(!overlap(a, &mark), "{a:?} covers the mark");
            for b in &rectangles[n + 1..] {
                assert!(!overlap(a, b), "{a:?} and {b:?} overlap");
            }
        }
    }

    #[test]
    fn a_label_never_leaves_the_scope() {
        let at_the_edge = place_labels(&[wanted(179, 100)], &[], &scope(), 5);
        let label = at_the_edge[0].expect("room on the inner side");
        assert!(label.top_left.x + 12 <= 179, "{label:?} put inside, to the left");
    }

    #[test]
    fn a_label_with_nowhere_to_go_is_left_out() {
        let wall = Rectangle::new(Point::new(20, 20), Size::new(160, 160));
        assert_eq!(place_labels(&[wanted(100, 100)], &[wall], &scope(), 5), [None]);
    }

    #[test]
    fn a_registration_is_called_by_its_last_two_letters() {
        assert_eq!(short_registration("F-GKXA").as_deref(), Some("XA"));
        assert_eq!(short_registration("N123AB").as_deref(), Some("AB"));
        assert_eq!(short_registration("G-"), None);
    }
}
