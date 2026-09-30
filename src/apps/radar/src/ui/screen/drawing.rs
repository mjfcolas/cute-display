use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle};
use ui::text::{self, HINT, LIST, TITLE};

use super::ui_state::{FromPlace, RadarUiState};
use crate::ui::radar_view::{aircraft_triangle, place_labels, LabelWanted, Scope};

const AIRPORT_DIAMETER: u32 = 4;
const TRACKLESS_AIRCRAFT_DIAMETER: u32 = 4;
const LABEL_CLEARANCE: i32 = AIRCRAFT_LENGTH / 2 + 2;
const HOME_ARM: i32 = 3;
const AIRCRAFT_LENGTH: i32 = 9;
const COLUMN_GAP: i32 = 10;
const ENTRY_PITCH: i32 = 26;
const LINE_GAP: i32 = 2;
const TICK: i32 = 4;
/// Required by adsb.fi's terms.
const ATTRIBUTION: &str = "data: adsb.fi";

pub(super) fn draw<D: DrawTarget<Color = BinaryColor>>(state: &RadarUiState, target: &mut D, area: Rectangle) {
    let diameter = (area.size.height as i32).min(area.size.width as i32 * 3 / 5);
    // An odd diameter, so the circle and its ticks land on the same pixels either side.
    let radius = (diameter - 1) / 2;
    let scope = Scope { center: area.top_left + Point::new(radius, radius), radius_px: radius, range_km: f64::from(state.range_km) };
    draw_scope(target, &scope, state);

    let column_left = area.top_left.x + diameter + COLUMN_GAP;
    let column = Rectangle::new(
        Point::new(column_left, area.top_left.y),
        Size::new((area.top_left.x + area.size.width as i32 - column_left).max(0) as u32, area.size.height),
    );
    draw_nearest(target, column, state);
    draw_footer(target, column, state);
}

fn draw_scope<D: DrawTarget<Color = BinaryColor>>(target: &mut D, scope: &Scope, state: &RadarUiState) {
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);
    let r = scope.radius_px;
    let _ = Circle::with_center(scope.center, (2 * r + 1) as u32).into_styled(stroke).draw(target);
    let _ = Circle::with_center(scope.center, r as u32).into_styled(stroke).draw(target);
    for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        let edge = scope.center + Point::new(dx * r, dy * r);
        let _ = Line::new(edge, edge - Point::new(dx * TICK, dy * TICK)).into_styled(stroke).draw(target);
        // The place itself is a small cross, so it is never taken for an airport.
        let arm = scope.center + Point::new(dx * HOME_ARM, dy * HOME_ARM);
        let _ = Line::new(scope.center, arm).into_styled(stroke).draw(target);
    }

    let locate = |from: FromPlace| scope.locate(from.east_km, from.north_km);
    let airports: Vec<(Option<&String>, Point)> = state.airports.iter().filter_map(|a| Some((a.label.as_ref(), locate(a.from_place)?))).collect();
    for &(_, at) in &airports {
        let _ = Circle::with_center(at, AIRPORT_DIAMETER).into_styled(fill).draw(target);
    }
    let located: Vec<(Option<&String>, Point, Option<f32>)> =
        state.aircraft.iter().filter_map(|a| Some((a.label.as_ref(), locate(a.from_place)?, a.track_degrees))).collect();
    for &(_, at, track) in &located {
        match track {
            Some(track) => {
                let [nose, left, right] = aircraft_triangle(at, track, AIRCRAFT_LENGTH);
                let _ = Triangle::new(nose, left, right).into_styled(fill).draw(target);
            }
            None => {
                let _ = Circle::with_center(at, TRACKLESS_AIRCRAFT_DIAMETER).into_styled(fill).draw(target);
            }
        }
    }
    let aircraft: Vec<(Option<&String>, Point)> = located.iter().map(|&(label, at, _)| (label, at)).collect();

    let mark = |at: Point, side: u32| Rectangle::with_center(at, Size::new(side, side));
    let mut obstacles: Vec<Rectangle> = aircraft.iter().map(|&(_, at)| mark(at, AIRCRAFT_LENGTH as u32)).collect();
    obstacles.extend(airports.iter().map(|&(_, at)| mark(at, AIRPORT_DIAMETER)));
    obstacles.push(mark(scope.center, (2 * HOME_ARM + 1) as u32));

    // Named airports first, since somebody asked for them.
    let labels: Vec<(&String, Point)> = airports.iter().chain(&aircraft).filter_map(|&(label, at)| Some((label?, at))).collect();
    let wanted: Vec<LabelWanted> = labels.iter().map(|(text, at)| LabelWanted { near: *at, size: label_size(text) }).collect();
    let placed = place_labels(&wanted, &obstacles, scope, LABEL_CLEARANCE);
    for ((text, _), spot) in labels.iter().zip(placed) {
        if let Some(spot) = spot {
            draw_label(target, text, spot);
        }
    }
}

fn label_size(text: &str) -> Size {
    let advance = HINT.character_size.width + HINT.character_spacing;
    Size::new(text.chars().count() as u32 * advance + 2, HINT.character_size.height + 2)
}

/// On a patch of paper, so a ring running through it does not cross its letters out.
fn draw_label<D: DrawTarget<Color = BinaryColor>>(target: &mut D, text: &str, spot: Rectangle) {
    let _ = spot.into_styled(PrimitiveStyle::with_fill(BinaryColor::Off)).draw(target);
    text::write(target, text, spot.top_left + Point::new(1, 1), spot.size.width, &HINT);
}

fn draw_nearest<D: DrawTarget<Color = BinaryColor>>(target: &mut D, column: Rectangle, state: &RadarUiState) {
    let x = column.top_left.x;
    let mut top = column.top_left.y;
    text::write(target, &state.title, Point::new(x, top), column.size.width, &TITLE);
    top += TITLE.character_size.height as i32 + LINE_GAP;
    text::write(target, &format!("range {} km", state.range_km), Point::new(x, top), column.size.width, &HINT);
    top += HINT.character_size.height as i32 + 3 * LINE_GAP;
    for (n, listed) in (0..).zip(&state.nearest) {
        let entry = top + n * ENTRY_PITCH;
        text::write(target, &listed.identity, Point::new(x, entry), column.size.width, &LIST);
        text::write(target, &listed.whereabouts, Point::new(x, entry + LIST.character_size.height as i32), column.size.width, &HINT);
    }
}

fn draw_footer<D: DrawTarget<Color = BinaryColor>>(target: &mut D, column: Rectangle, state: &RadarUiState) {
    let lines = footer(state.trouble.as_deref(), text::chars_across(column.size.width, &HINT));
    let pitch = HINT.character_size.height as i32 + 1;
    let bottom = column.top_left.y + column.size.height as i32;
    for (n, line) in lines.iter().enumerate() {
        let top = bottom - (lines.len() - n) as i32 * pitch;
        text::write(target, line, Point::new(column.top_left.x, top), column.size.width, &HINT);
    }
}

fn footer(trouble: Option<&str>, chars: usize) -> Vec<String> {
    let mut lines = trouble.map_or_else(Vec::new, |trouble| text::wrap(trouble, chars));
    lines.extend(["wheel: range", "long: update", ATTRIBUTION].map(str::to_owned));
    lines
}

#[cfg(test)]
mod tests {
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::ui::screen::{AircraftMark, AirportMark, ListedAircraft, NEAREST_LISTED};

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn plane(east_km: f64, north_km: f64) -> AircraftMark {
        AircraftMark { from_place: FromPlace { east_km, north_km }, track_degrees: Some(45.0), label: Some("XA".into()) }
    }

    fn unlabelled(aircraft: AircraftMark) -> AircraftMark {
        AircraftMark { label: None, ..aircraft }
    }

    fn orly(label: Option<&str>) -> AirportMark {
        AirportMark { from_place: FromPlace { east_km: -6.0, north_km: -15.0 }, label: label.map(str::to_owned) }
    }

    fn state(aircraft: Vec<AircraftMark>, airports: Vec<AirportMark>) -> RadarUiState {
        let listed = ListedAircraft { identity: "XA AFR1234".into(), whereabouts: "   35000 ft  3.0 km".into() };
        RadarUiState {
            title: "Notre-Dame".into(),
            range_km: 25,
            airports,
            nearest: vec![listed; aircraft.len().min(NEAREST_LISTED)],
            aircraft,
            trouble: Some("offline: no Wi-Fi: put cute-display/wifi.conf".into()),
        }
    }

    fn render(state: &RadarUiState) -> Frame {
        let mut frame = Frame::blank();
        draw(state, &mut frame, AREA);
        frame
    }

    fn scope_center() -> Point {
        let radius = ((AREA.size.height as i32).min(AREA.size.width as i32 * 3 / 5) - 1) / 2;
        AREA.top_left + Point::new(radius, radius)
    }

    fn ink_in(frame: &Frame, top_left: Point, size: i32) -> usize {
        (0..size).flat_map(|dy| (0..size).map(move |dx| (dx, dy))).filter(|&(dx, dy)| frame.is_ink(top_left.x + dx, top_left.y + dy)).count()
    }

    #[test]
    fn an_aircraft_to_the_north_east_is_drawn_up_and_to_the_right() {
        let empty = render(&state(vec![], vec![]));
        let with_plane = render(&state(vec![unlabelled(plane(8.0, 8.0))], vec![]));
        let c = scope_center();
        let added = |dx: i32, dy: i32| {
            let corner = c + Point::new(dx * 30 - 10, dy * 30 - 10);
            ink_in(&with_plane, corner, 20) - ink_in(&empty, corner, 20)
        };
        assert!(added(1, -1) > 0, "north-east");
        assert_eq!(added(-1, 1) + added(1, 1) + added(-1, -1), 0, "nowhere else");
    }

    #[test]
    fn aircraft_carry_their_labels_even_in_a_crowd() {
        let crowd: Vec<AircraftMark> = (0..12).map(|n| plane(f64::from(n) * 2.0 - 12.0, 4.0)).collect();
        let labelled = render(&state(crowd.clone(), vec![]));
        let unlabelled = render(&state(crowd.into_iter().map(unlabelled).collect(), vec![]));
        assert!(labelled != unlabelled, "some labels found room");
    }

    #[test]
    fn an_airport_is_a_dot_where_it_lies_and_its_label_is_drawn() {
        let empty = render(&state(vec![], vec![]));
        let quiet = render(&state(vec![], vec![orly(None)]));
        let corner = scope_center() + Point::new(-40, 30);
        assert!(ink_in(&quiet, corner, 40) > ink_in(&empty, corner, 40), "south-west of the center");
        assert!(render(&state(vec![], vec![orly(Some("LFPO"))])) != quiet);
    }

    #[test]
    fn trouble_is_said_above_the_controls_in_lines_that_fit() {
        let lines = footer(Some("no place: put cute-display/general.conf"), 22);
        assert_eq!(lines, ["no place: put", "cute-display/general.c", "onf", "wheel: range", "long: update", ATTRIBUTION]);
        assert!(lines.iter().all(|l| l.chars().count() <= 22));
        assert_eq!(footer(None, 22), ["wheel: range", "long: update", ATTRIBUTION], "the source is credited");
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let mut busy: Vec<AircraftMark> = (0..40).map(|n| plane(f64::from(n) - 20.0, f64::from(n % 7) * 3.0 - 9.0)).collect();
        busy.push(AircraftMark { track_degrees: None, ..plane(-2.0, 18.0) });
        busy.push(plane(30.0, 0.0));
        let frame = render(&state(busy, vec![orly(Some("LFPO"))]));
        for x in 0..i32::from(WIDTH) {
            for y in 0..i32::from(HEIGHT) {
                if frame.is_ink(x, y) {
                    assert!(AREA.contains(Point::new(x, y)) && x < i32::from(VISIBLE_WIDTH), "ink at ({x},{y})");
                }
            }
        }
    }
}
