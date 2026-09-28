//! A scope on the left, the nearest aircraft on the right.

use domain::fetch::FetchStatus;
use domain::place::GeoPoint;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle};
use ui::controls::{Button, Input};
use ui::text::{self, HINT, LIST, TITLE};
use ui::AppScreen;

use crate::domain::radar::{Aircraft, Altitude, Radar, RadarReport};
use crate::ui::radar_view::{aircraft_triangle, place_labels, short_registration, LabelWanted, Scope};

const NEAREST_LISTED: usize = 5;
const AIRPORT_DIAMETER: u32 = 4;
const TRACKLESS_AIRCRAFT_DIAMETER: u32 = 4;
/// From the center of a mark to its label.
const LABEL_CLEARANCE: i32 = AIRCRAFT_LENGTH / 2 + 2;
const HOME_ARM: i32 = 3;
const AIRCRAFT_LENGTH: i32 = 9;
const COLUMN_GAP: i32 = 10;
/// Two lines an aircraft: who it is, then where.
const ENTRY_PITCH: i32 = 26;
const LINE_GAP: i32 = 2;
const TICK: i32 = 4;
/// Required by adsb.fi's terms.
const ATTRIBUTION: &str = "data: adsb.fi";

pub struct RadarScreen {
    radar: Radar,
}

impl RadarScreen {
    pub fn new(radar: Radar) -> Self {
        Self { radar }
    }
}

impl<D: DrawTarget<Color = BinaryColor>> AppScreen<D> for RadarScreen {
    fn version(&self) -> u64 {
        self.radar.report().revision
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::Turn(detents) => {
                let mut range = self.radar.report().range;
                for _ in 0..detents.unsigned_abs() {
                    range = if detents > 0 { range.wider() } else { range.closer() };
                }
                self.radar.set_range(range);
            }
            Input::Press(Button::Long) => self.radar.request_refresh(),
            Input::Press(Button::Yellow) | Input::HoldYellowAndLong => {}
        }
    }

    /// The scope takes the whole height; everything written goes in the column beside it.
    fn draw(&self, target: &mut D, area: Rectangle) {
        let report = self.radar.report();
        let diameter = (area.size.height as i32).min(area.size.width as i32 * 3 / 5);
        // An odd diameter, so the circle and its ticks land on the same pixels either side.
        let radius = (diameter - 1) / 2;
        let scope = Scope {
            center: area.top_left + Point::new(radius, radius),
            radius_px: radius,
            range_km: f64::from(report.range.km()),
        };
        draw_scope(target, &scope, &report);

        let column_left = area.top_left.x + diameter + COLUMN_GAP;
        let column = Rectangle::new(
            Point::new(column_left, area.top_left.y),
            Size::new((area.top_left.x + area.size.width as i32 - column_left).max(0) as u32, area.size.height),
        );
        draw_nearest(target, column, &report);
        draw_footer(target, column, &report);
    }
}

fn draw_scope<D: DrawTarget<Color = BinaryColor>>(target: &mut D, scope: &Scope, report: &RadarReport) {
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

    let Some(center) = report.center else { return };
    let on_scope = |point: GeoPoint| {
        let (east, north) = point.offset_from(center);
        scope.locate(east, north)
    };
    for airport in &report.airports {
        if let Some(at) = on_scope(airport.point) {
            let _ = Circle::with_center(at, AIRPORT_DIAMETER).into_styled(fill).draw(target);
        }
    }

    let shown: Vec<(&Aircraft, Point)> = report.aircraft.iter().filter_map(|a| Some((a, on_scope(a.point)?))).collect();
    for &(aircraft, at) in &shown {
        match aircraft.track_degrees {
            Some(track) => {
                let [nose, left, right] = aircraft_triangle(at, track, AIRCRAFT_LENGTH);
                let _ = Triangle::new(nose, left, right).into_styled(fill).draw(target);
            }
            None => {
                let _ = Circle::with_center(at, TRACKLESS_AIRCRAFT_DIAMETER).into_styled(fill).draw(target);
            }
        }
    }

    // What labels must not cover: every mark on the scope.
    let mark = |at: Point, side: u32| Rectangle::with_center(at, Size::new(side, side));
    let mut obstacles: Vec<Rectangle> = shown.iter().map(|&(_, at)| mark(at, AIRCRAFT_LENGTH as u32)).collect();
    obstacles.extend(report.airports.iter().filter_map(|a| on_scope(a.point)).map(|at| mark(at, AIRPORT_DIAMETER)));
    obstacles.push(mark(scope.center, (2 * HOME_ARM + 1) as u32));

    // Named airports first, since somebody asked for them; then aircraft, nearest first.
    let named_airports = report.airports.iter().filter(|a| a.labelled).filter_map(|a| Some((a.code.clone(), on_scope(a.point)?)));
    let registrations = shown.iter().filter_map(|&(a, at)| Some((short_registration(a.registration.as_deref()?)?, at)));
    let labels: Vec<(String, Point)> = named_airports.chain(registrations).collect();
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

fn draw_nearest<D: DrawTarget<Color = BinaryColor>>(target: &mut D, column: Rectangle, report: &RadarReport) {
    let x = column.top_left.x;
    let mut top = column.top_left.y;
    text::write(target, report.place.as_deref().unwrap_or("Radar"), Point::new(x, top), column.size.width, &TITLE);
    top += TITLE.character_size.height as i32 + LINE_GAP;
    text::write(target, &format!("range {} km", report.range.km()), Point::new(x, top), column.size.width, &HINT);
    top += HINT.character_size.height as i32 + 3 * LINE_GAP;

    let Some(center) = report.center else { return };
    let within = report.aircraft.iter().filter(|a| a.point.distance_km(center) <= f64::from(report.range.km()));
    for (n, aircraft) in within.take(NEAREST_LISTED).enumerate() {
        let entry = top + n as i32 * ENTRY_PITCH;
        text::write(target, &identity(aircraft), Point::new(x, entry), column.size.width, &LIST);
        let whereabouts = whereabouts(aircraft, aircraft.point.distance_km(center));
        text::write(target, &whereabouts, Point::new(x, entry + LIST.character_size.height as i32), column.size.width, &HINT);
    }
}

/// At the foot of the column: what is wrong if anything is, the controls, the source.
fn draw_footer<D: DrawTarget<Color = BinaryColor>>(target: &mut D, column: Rectangle, report: &RadarReport) {
    let lines = footer(report, text::chars_across(column.size.width, &HINT));
    let pitch = HINT.character_size.height as i32 + 1;
    let bottom = column.top_left.y + column.size.height as i32;
    for (n, line) in lines.iter().enumerate() {
        let top = bottom - (lines.len() - n) as i32 * pitch;
        text::write(target, line, Point::new(column.top_left.x, top), column.size.width, &HINT);
    }
}

/// Its two letters first, as on the scope, so the line and the mark can be matched.
fn identity(aircraft: &Aircraft) -> String {
    let letters = aircraft.registration.as_deref().and_then(short_registration).unwrap_or_default();
    format!("{letters:<2} {}", aircraft.callsign.as_deref().unwrap_or("?"))
}

fn whereabouts(aircraft: &Aircraft, distance_km: f64) -> String {
    let altitude = match aircraft.altitude {
        Some(Altitude::Ground) => "ground".to_owned(),
        Some(Altitude::Feet(feet)) => format!("{feet} ft"),
        None => "-".to_owned(),
    };
    let distance = if distance_km < 10.0 { format!("{distance_km:.1} km") } else { format!("{distance_km:.0} km") };
    format!("   {altitude}  {distance}")
}

fn footer(report: &RadarReport, chars: usize) -> Vec<String> {
    let trouble = match &report.status {
        FetchStatus::NeverFetched | FetchStatus::Updating if report.fetched_at.is_none() => "looking...".to_owned(),
        FetchStatus::NoPlace => "no place: put cute-display/general.conf".to_owned(),
        FetchStatus::Failed(why) => format!("offline: {why}"),
        _ => String::new(),
    };
    let mut lines = text::wrap(&trouble, chars);
    lines.extend(["wheel: range", "long: update", ATTRIBUTION].map(str::to_owned));
    lines
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use domain::apps::Foreground;
    use domain::fetch::Unavailable;
    use domain::place::{Place, PlaceSource};
    use hal::display::{Frame, HEIGHT, VISIBLE_WIDTH, WIDTH};

    use super::*;
    use crate::domain::radar::{AirTrafficSource, Airport, AirportSource, Range};

    const NOTRE_DAME: GeoPoint = GeoPoint { latitude: 48.8530, longitude: 2.3499 };

    struct Nowhere;

    impl PlaceSource for Nowhere {
        fn place(&mut self) -> Option<Place> {
            None
        }
    }

    struct NotreDame;

    impl PlaceSource for NotreDame {
        fn place(&mut self) -> Option<Place> {
            Some(Place { name: "Notre-Dame".into(), point: NOTRE_DAME })
        }
    }

    #[derive(Clone)]
    struct Traffic(Arc<Mutex<Vec<Aircraft>>>);

    impl AirTrafficSource for Traffic {
        fn nearby(&mut self, _: GeoPoint, _: u32) -> Result<Vec<Aircraft>, Unavailable> {
            Ok(self.0.lock().unwrap().clone())
        }
    }

    fn at(east_km: f64, north_km: f64) -> GeoPoint {
        GeoPoint {
            latitude: NOTRE_DAME.latitude + north_km / 111.2,
            longitude: NOTRE_DAME.longitude + east_km / (111.2 * NOTRE_DAME.latitude.to_radians().cos()),
        }
    }

    fn plane(east_km: f64, north_km: f64) -> Aircraft {
        Aircraft {
            callsign: Some("AFR1234".into()),
            registration: Some("F-GKXA".into()),
            point: at(east_km, north_km),
            altitude: Some(Altitude::Feet(35_000)),
            track_degrees: Some(45.0),
        }
    }

    fn unnamed(aircraft: Aircraft) -> Aircraft {
        Aircraft { callsign: None, registration: None, ..aircraft }
    }

    struct Airports(Vec<Airport>);

    impl AirportSource for Airports {
        fn airports(&mut self) -> Vec<Airport> {
            self.0.clone()
        }
    }

    fn radar_with_airports(traffic: Vec<Aircraft>, airports: Vec<Airport>) -> Radar {
        let traffic = Box::new(Traffic(Arc::new(Mutex::new(traffic))));
        let radar = Radar::new(Box::new(NotreDame), traffic, Box::new(Airports(airports)), Foreground::new(crate::ID));
        radar.refresh_if_due(Instant::now());
        radar
    }

    fn radar_with(traffic: Vec<Aircraft>) -> Radar {
        radar_with_airports(traffic, vec![])
    }

    const AREA: Rectangle = Rectangle::new(Point::new(8, 8), Size::new(382, 224));

    fn render(screen: &RadarScreen) -> Frame {
        let mut frame = Frame::blank();
        AppScreen::<Frame>::draw(screen, &mut frame, AREA);
        frame
    }

    fn scope_center() -> Point {
        let radius = ((AREA.size.height as i32).min(AREA.size.width as i32 * 3 / 5) - 1) / 2;
        AREA.top_left + Point::new(radius, radius)
    }

    /// Ink in the square of `size` pixels at `top_left`.
    fn ink_in(frame: &Frame, top_left: Point, size: i32) -> usize {
        (0..size).flat_map(|dy| (0..size).map(move |dx| (dx, dy))).filter(|&(dx, dy)| frame.is_ink(top_left.x + dx, top_left.y + dy)).count()
    }

    #[test]
    fn an_aircraft_to_the_north_east_is_drawn_up_and_to_the_right() {
        let empty = render(&RadarScreen::new(radar_with(vec![])));
        let with_plane = render(&RadarScreen::new(radar_with(vec![plane(8.0, 8.0)])));
        let c = scope_center();
        let added = |dx: i32, dy: i32| {
            let corner = c + Point::new(dx * 30 - 10, dy * 30 - 10);
            ink_in(&with_plane, corner, 20) - ink_in(&empty, corner, 20)
        };
        assert!(added(1, -1) > 0, "north-east");
        assert_eq!(added(-1, 1) + added(1, 1) + added(-1, -1), 0, "nowhere else");
    }

    #[test]
    fn aircraft_carry_their_registration_even_in_a_crowd() {
        let crowd: Vec<Aircraft> = (0..12).map(|n| plane(f64::from(n) * 2.0 - 12.0, 4.0)).collect();
        let named = render(&RadarScreen::new(radar_with(crowd.clone())));
        let anonymous = render(&RadarScreen::new(radar_with(crowd.into_iter().map(unnamed).collect())));
        assert!(named != anonymous, "some labels found room");
    }

    #[test]
    fn an_airport_is_named_only_when_asked() {
        let orly = |labelled| Airport { code: "LFPO".into(), point: at(-6.0, -15.0), labelled };
        let quiet = render(&RadarScreen::new(radar_with_airports(vec![], vec![orly(false)])));
        let named = render(&RadarScreen::new(radar_with_airports(vec![], vec![orly(true)])));
        assert!(named != quiet);
    }

    #[test]
    fn an_airport_is_a_dot_where_it_lies() {
        let empty = render(&RadarScreen::new(radar_with(vec![])));
        let orly = Airport { code: "LFPO".into(), point: at(-6.0, -15.0), labelled: false };
        let with_airport = render(&RadarScreen::new(radar_with_airports(vec![], vec![orly])));
        let c = scope_center();
        let corner = c + Point::new(-40, 30);
        assert!(ink_in(&with_airport, corner, 40) > ink_in(&empty, corner, 40), "south-west of the center");
    }

    #[test]
    fn an_aircraft_beyond_the_range_is_not_on_the_scope() {
        let empty = render(&RadarScreen::new(radar_with(vec![])));
        let far = RadarScreen::new(radar_with(vec![plane(40.0, 0.0)]));
        assert_eq!(far.radar.report().range, Range::TwentyFiveKm);
        let frame = render(&far);
        let c = scope_center();
        let scope_box = |f: &Frame| (0..2 * c.x).flat_map(|x| (AREA.top_left.y..2 * c.y).map(move |y| (x, y))).filter(|&(x, y)| f.is_ink(x, y)).count();
        assert_eq!(scope_box(&frame), scope_box(&empty));
    }

    #[test]
    fn the_wheel_changes_the_range_and_stops_at_its_ends() {
        let mut screen = RadarScreen::new(radar_with(vec![]));
        AppScreen::<Frame>::on_input(&mut screen, Input::Turn(1));
        assert_eq!(screen.radar.report().range, Range::FiftyKm);
        AppScreen::<Frame>::on_input(&mut screen, Input::Turn(10));
        assert_eq!(screen.radar.report().range, Range::HundredKm);
        AppScreen::<Frame>::on_input(&mut screen, Input::Turn(-10));
        assert_eq!(screen.radar.report().range, Range::FiveKm);
    }

    #[test]
    fn a_press_asks_for_an_update() {
        let radar = radar_with(vec![]);
        let now = Instant::now();
        let mut screen = RadarScreen::new(radar.clone());
        assert!(!radar.is_due(now));
        AppScreen::<Frame>::on_input(&mut screen, Input::Press(Button::Long));
        assert!(radar.is_due(now));
    }

    #[test]
    fn the_nearest_are_listed_and_the_source_is_credited() {
        let radar = radar_with(vec![plane(3.0, 0.0), plane(60.0, 0.0)]);
        let report = radar.report();
        assert_eq!(footer(&report, 24), ["wheel: range", "long: update", ATTRIBUTION]);
        assert_eq!(identity(&report.aircraft[0]), "XA AFR1234");
        assert_eq!(identity(&unnamed(report.aircraft[0].clone())), "   ?");
        assert_eq!(whereabouts(&report.aircraft[0], 3.04), "   35000 ft  3.0 km");
    }

    #[test]
    fn trouble_is_said_above_the_controls_in_lines_that_fit() {
        let no_place = Radar::new(
            Box::new(Nowhere),
            Box::new(Traffic(Arc::default())),
            Box::new(Airports(vec![])),
            Foreground::new(crate::ID),
        );
        no_place.refresh_if_due(Instant::now());
        let lines = footer(&no_place.report(), 22);
        assert_eq!(lines, ["no place: put", "cute-display/general.c", "onf", "wheel: range", "long: update", ATTRIBUTION]);
        assert!(lines.iter().all(|l| l.chars().count() <= 22));
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let busy: Vec<Aircraft> = (0..40).map(|n| plane(f64::from(n) - 20.0, f64::from(n % 7) * 3.0 - 9.0)).collect();
        let frame = render(&RadarScreen::new(radar_with(busy)));
        for x in 0..i32::from(WIDTH) {
            for y in 0..i32::from(HEIGHT) {
                if frame.is_ink(x, y) {
                    assert!(AREA.contains(Point::new(x, y)) && x < i32::from(VISIBLE_WIDTH), "ink at ({x},{y})");
                }
            }
        }
    }
}
