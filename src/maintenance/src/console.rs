use std::io::{self, Write};

use hal::clock::RealTimeClock;
use hal::display::FRAME_BYTES;
use hal::storage::{CopyOutcome, FileStorage};
use hal::Fault;

use crate::path::SdPath;
use crate::observation::{LightName, Observation};
use crate::protocol::{self, CardRequest, ClockRequest, ObservationRequest, RemoteRequest, Request};
use crate::remote::Remote;

/// A put is for configuration, not for media.
pub const MAX_PUT_BYTES: usize = 64 * 1024;
/// The most a get sends: the computer asks again from where it stopped, so a file of any
/// size is never held whole.
pub const GET_RANGE_BYTES: usize = 8 * 1024;
/// Bytes per `data` line: 320 characters once in base64.
const DATA_CHUNK_BYTES: usize = 240;

struct PendingPut {
    id: String,
    path: SdPath,
    size_bytes: usize,
    crc32: u32,
    received: Vec<u8>,
}

pub struct MaintenanceConsole<S> {
    storage: Result<S, Fault>,
    remote: Remote,
    observation: Observation,
    rtc: Box<dyn RealTimeClock + Send>,
    put: Option<PendingPut>,
}

impl<S: FileStorage> MaintenanceConsole<S> {
    pub fn new(storage: Result<S, Fault>, remote: Remote, observation: Observation, rtc: impl RealTimeClock + Send + 'static) -> Self {
        Self { storage, remote, observation, rtc: Box::new(rtc), put: None }
    }

    /// One line from the computer. Lines that are not the protocol's are ignored; a new
    /// request in the middle of a put abandons it.
    pub fn on_line(&mut self, line: &str, out: &mut impl Write) -> io::Result<()> {
        let Some((id, request)) = protocol::parse(line) else {
            return Ok(());
        };
        let replies = match request {
            Err(reason) => vec![protocol::error(&id, &reason)],
            Ok(Request::Card(CardRequest::Data(bytes))) => self.receive(&id, &bytes),
            Ok(Request::Card(CardRequest::End)) => self.finish_put(&id),
            Ok(request) => {
                self.put = None;
                match request {
                    Request::Card(request) => self.serve_card(&id, request),
                    Request::Remote(request) => self.serve_remote(&id, request),
                    Request::Observation(request) => return self.serve_observation(&id, request, out),
                    Request::Clock(request) => self.serve_clock(&id, request),
                }
            }
        };
        for reply in replies {
            writeln!(out, "{reply}")?;
        }
        out.flush()
    }

    fn serve_remote(&self, id: &str, request: RemoteRequest) -> Vec<String> {
        match request {
            RemoteRequest::Tap(button) => self.remote.tap(button),
            RemoteRequest::Hold { buttons, duration } => self.remote.hold(&buttons, duration),
            RemoteRequest::Turn(clockwise_detents) => self.remote.turn(clockwise_detents),
        }
        vec![protocol::ok(id, "")]
    }

    /// A line at a time, so that neither the frame nor its lines are held whole in the
    /// internal RAM a request would take them from.
    fn send_screen(&self, id: &str, out: &mut impl Write) -> io::Result<()> {
        let mut chunk = [0; DATA_CHUNK_BYTES];
        let mut times_shown = None;
        for offset in (0..FRAME_BYTES).step_by(DATA_CHUNK_BYTES) {
            let part = chunk.get_mut(..DATA_CHUNK_BYTES.min(FRAME_BYTES - offset)).unwrap_or_default();
            let now = self.observation.copy_screen(offset, part);
            let failed = match (times_shown, now) {
                (_, None) => Some("nothing shown yet"),
                (Some(before), Some(now)) if before != now => Some("the screen changed while it was sent"),
                _ => None,
            };
            if let Some(reason) = failed {
                writeln!(out, "{}", protocol::error(id, reason))?;
                return out.flush();
            }
            times_shown = now;
            writeln!(out, "{}", protocol::data(id, part))?;
        }
        writeln!(out, "{}", protocol::ok(id, &times_shown.unwrap_or_default().to_string()))?;
        out.flush()
    }

    fn serve_observation(&self, id: &str, request: ObservationRequest, out: &mut impl Write) -> io::Result<()> {
        let observation = &self.observation;
        let reply = match request {
            ObservationRequest::Screen => return self.send_screen(id, out),
            ObservationRequest::Lights => {
                let percent = |name| observation.brightness(name).as_percent();
                protocol::ok(id, &format!("{} {}", percent(LightName::FrontLight), percent(LightName::ReadingLamp)))
            }
            ObservationRequest::Sound => protocol::ok(id, if observation.is_playing() { "playing" } else { "silent" }),
        };
        writeln!(out, "{reply}")?;
        out.flush()
    }

    fn serve_clock(&mut self, id: &str, request: ClockRequest) -> Vec<String> {
        match request {
            ClockRequest::Read => vec![match self.rtc.read() {
                Ok(reading) => protocol::ok(id, &reading.time.unix_seconds().to_string()),
                Err(fault) => protocol::error(id, fault.reason()),
            }],
            ClockRequest::Set(time) => vec![match self.rtc.set(time) {
                Ok(()) => protocol::ok(id, ""),
                Err(fault) => protocol::error(id, fault.reason()),
            }],
        }
    }

    fn serve_card(&mut self, id: &str, request: CardRequest) -> Vec<String> {
        let storage = match inserted_card(&self.storage, id) {
            Ok(storage) => storage,
            Err(replies) => return replies,
        };
        match request {
            CardRequest::List(dir) => match storage.entries(dir.as_str()) {
                Ok(Some(entries)) => entries
                    .iter()
                    .map(|e| protocol::entry(id, e))
                    .chain([protocol::ok(id, "")])
                    .collect(),
                Ok(None) => vec![protocol::error(id, "no such directory")],
                Err(fault) => vec![protocol::error(id, fault.reason())],
            },
            CardRequest::Get { path, offset } => match storage.read_range(path.as_str(), offset, GET_RANGE_BYTES) {
                Ok(Some(contents)) => contents
                    .chunks(DATA_CHUNK_BYTES)
                    .map(|chunk| protocol::data(id, chunk))
                    .chain([protocol::ok(id, &format!("{} {}", contents.len(), crc32fast::hash(&contents)))])
                    .collect(),
                Ok(None) => vec![protocol::error(id, "no such file")],
                Err(fault) => vec![protocol::error(id, fault.reason())],
            },
            CardRequest::Put { path, size_bytes, crc32 } => {
                if !path.is_writable() {
                    return vec![protocol::error(id, "only cute-display/ may be written")];
                }
                if size_bytes > MAX_PUT_BYTES {
                    return vec![protocol::error(id, &format!("larger than {MAX_PUT_BYTES} bytes"))];
                }
                self.put = Some(PendingPut { id: id.to_owned(), path, size_bytes, crc32, received: Vec::new() });
                vec![protocol::ready(id)]
            }
            CardRequest::Remove(path) => {
                if !path.is_writable() {
                    return vec![protocol::error(id, "only cute-display/ may be written")];
                }
                match storage.remove(path.as_str()) {
                    Ok(()) => vec![protocol::ok(id, "")],
                    Err(fault) => vec![protocol::error(id, fault.reason())],
                }
            }
            CardRequest::Copy { from, to } => {
                if !to.is_writable() {
                    return vec![protocol::error(id, "only cute-display/ may be written")];
                }
                match storage.copy(from.as_str(), to.as_str()) {
                    Ok(CopyOutcome::Copied) => vec![protocol::ok(id, "")],
                    Ok(CopyOutcome::NoSource) => vec![protocol::error(id, "no such file")],
                    Err(fault) => vec![protocol::error(id, fault.reason())],
                }
            }
            CardRequest::Data(_) | CardRequest::End => vec![protocol::error(id, "no put in progress")],
        }
    }

    fn receive(&mut self, id: &str, bytes: &[u8]) -> Vec<String> {
        let Some(put) = self.put.as_mut().filter(|put| put.id == id) else {
            return vec![protocol::error(id, "no put in progress")];
        };
        put.received.extend_from_slice(bytes);
        if put.received.len() > put.size_bytes {
            self.put = None;
            return vec![protocol::error(id, "more data than announced")];
        }
        vec![protocol::ack(id)]
    }

    fn finish_put(&mut self, id: &str) -> Vec<String> {
        let Some(put) = self.put.take().filter(|put| put.id == id) else {
            return vec![protocol::error(id, "no put in progress")];
        };
        if put.received.len() != put.size_bytes {
            return vec![protocol::error(id, &format!("received {} bytes of {}", put.received.len(), put.size_bytes))];
        }
        if crc32fast::hash(&put.received) != put.crc32 {
            return vec![protocol::error(id, "the CRC does not match; nothing was written")];
        }
        let storage = match inserted_card(&self.storage, id) {
            Ok(storage) => storage,
            Err(replies) => return replies,
        };
        match storage.write(put.path.as_str(), &put.received) {
            Ok(()) => vec![protocol::ok(id, "")],
            Err(fault) => vec![protocol::error(id, fault.reason())],
        }
    }
}

fn inserted_card<'a, S>(storage: &'a Result<S, Fault>, id: &str) -> Result<&'a S, Vec<String>> {
    storage.as_ref().map_err(|fault| vec![protocol::error(id, &format!("no SD card: {fault}"))])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use core::time::Duration;
    use std::time::Instant;

    use hal::input::{PushButton, RotaryEncoder};
    use hal::steady::SteadyClock;
    use hal::storage::Entry;
    use hal::Fault;

    use super::*;
    use hal::clock::{ClockReading, DateTime};
    use hal::display::{EpaperDisplay, Frame, Redraw, Refreshed, FRAME_BYTES};
    use hal::light::{Brightness, DimmableLight};

    use crate::remote::ButtonName;

    #[derive(Clone, Default)]
    struct FakeCard(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

    impl FileStorage for FakeCard {
        fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
            let prefix = if dir.is_empty() { String::new() } else { format!("{dir}/") };
            let entries: Vec<Entry> = self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|(path, contents)| {
                    let name = path.strip_prefix(&prefix)?;
                    Some(Entry { name: name.into(), size_bytes: contents.len() as u64, is_dir: false })
                })
                .collect();
            Ok((dir.is_empty() || !entries.is_empty()).then_some(entries))
        }
        fn capacity_bytes(&self) -> Result<u64, Fault> {
            Ok(0)
        }
        fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
            Ok(self.0.lock().unwrap().get(path).cloned())
        }
        fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
            self.0.lock().unwrap().insert(path.into(), contents.to_vec());
            Ok(())
        }
        fn remove(&self, path: &str) -> Result<(), Fault> {
            self.0.lock().unwrap().remove(path);
            Ok(())
        }
    }

    impl FakeCard {
        fn with(path: &str, contents: &[u8]) -> Self {
            let card = Self::default();
            card.0.lock().unwrap().insert(path.into(), contents.to_vec());
            card
        }

        fn file(&self, path: &str) -> Option<Vec<u8>> {
            self.0.lock().unwrap().get(path).cloned()
        }
    }

    const MORNING: DateTime = DateTime { year: 2026, month: 9, day: 26, hour: 7, minute: 30, second: 15 };

    struct Stopped(DateTime);
    impl RealTimeClock for Stopped {
        fn read(&mut self) -> Result<ClockReading, Fault> {
            Ok(ClockReading { time: self.0, oscillator_stopped: false, alarm_raised: false })
        }
        fn set(&mut self, time: DateTime) -> Result<(), Fault> {
            self.0 = time;
            Ok(())
        }
    }

    struct NoWait;
    impl SteadyClock for NoWait {
        fn now(&self) -> Instant {
            Instant::now()
        }
        fn sleep(&self, _: Duration) {}
    }

    fn console(card: FakeCard) -> MaintenanceConsole<FakeCard> {
        MaintenanceConsole::new(Ok(card), Remote::new(NoWait), Observation::default(), Stopped(MORNING))
    }

    fn talk(console: &mut MaintenanceConsole<FakeCard>, lines: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for line in lines {
            console.on_line(line, &mut out).unwrap();
        }
        String::from_utf8(out)
            .unwrap()
            .lines()
            .filter(|l| !l.ends_with(" ready") && !l.ends_with(" ack"))
            .map(str::to_owned)
            .collect()
    }

    fn put_lines(id: &str, path: &str, contents: &[u8], crc32: u32) -> Vec<String> {
        let mut lines = vec![format!("@@ {id} put {} {crc32} {path}", contents.len())];
        lines.extend(contents.chunks(10).map(|c| format!("@@ {id} data {}", STANDARD.encode(c))));
        lines.push(format!("@@ {id} end"));
        lines
    }

    fn decode_get(replies: &[String], id: &str) -> Vec<u8> {
        let prefix = format!("@@ {id} data ");
        replies.iter().filter_map(|r| r.strip_prefix(&prefix)).flat_map(|b64| STANDARD.decode(b64).unwrap()).collect()
    }

    #[test]
    fn a_file_put_comes_back_byte_for_byte() {
        let card = FakeCard::default();
        let mut console = console(card.clone());
        let contents: Vec<u8> = (0..=255u8).cycle().take(1000).collect();

        let replies = talk(&mut console, &put_lines("1", "cute-display/wifi.conf", &contents, crc32fast::hash(&contents)));
        assert_eq!(replies, ["@@ 1 ok"]);
        assert_eq!(card.file("cute-display/wifi.conf").as_deref(), Some(contents.as_slice()));

        let replies = talk(&mut console, &["@@ 2 get 0 cute-display/wifi.conf".into()]);
        assert_eq!(decode_get(&replies, "2"), contents);
        assert_eq!(replies.last().unwrap(), &format!("@@ 2 ok 1000 {}", crc32fast::hash(&contents)));
    }

    #[test]
    fn a_put_is_answered_line_by_line_so_the_computer_never_outruns_the_console() {
        let mut console = console(FakeCard::default());
        let lines = put_lines("5", "cute-display/a.conf", b"0123456789abcdefghij", crc32fast::hash(b"0123456789abcdefghij"));
        let mut replies = Vec::new();
        for line in &lines {
            let mut out = Vec::new();
            console.on_line(line, &mut out).unwrap();
            replies.push(String::from_utf8(out).unwrap().trim_end().to_owned());
        }
        assert_eq!(replies, ["@@ 5 ready", "@@ 5 ack", "@@ 5 ack", "@@ 5 ok"]);
    }

    #[test]
    fn a_put_with_the_wrong_crc_writes_nothing() {
        let card = FakeCard::default();
        let mut console = console(card.clone());
        let replies = talk(&mut console, &put_lines("1", "cute-display/a.conf", b"hello", 42));
        assert!(replies[0].starts_with("@@ 1 error"));
        assert_eq!(card.file("cute-display/a.conf"), None);
    }

    #[test]
    fn nothing_outside_the_devices_directory_is_written_or_removed() {
        let card = FakeCard::with("sounds/alarm.wav", b"RIFF");
        let mut console = console(card.clone());
        let replies = talk(&mut console, &put_lines("1", "sounds/alarm.wav", b"oops", crc32fast::hash(b"oops")));
        assert!(replies[0].starts_with("@@ 1 error"));
        let replies = talk(&mut console, &["@@ 2 rm sounds/alarm.wav".into()]);
        assert!(replies[0].starts_with("@@ 2 error"));
        assert_eq!(card.file("sounds/alarm.wav").as_deref(), Some(b"RIFF".as_slice()));
    }

    #[test]
    fn anything_may_be_listed_and_read() {
        let card = FakeCard::with("sounds/alarm.wav", b"RIFF");
        let mut console = console(card);
        let replies = talk(&mut console, &["@@ 1 ls sounds".into(), "@@ 2 get 0 sounds/alarm.wav".into()]);
        assert_eq!(replies[0], "@@ 1 entry f 4 alarm.wav");
        assert_eq!(replies[1], "@@ 1 ok");
        assert_eq!(decode_get(&replies, "2"), b"RIFF");
    }

    #[test]
    fn a_directory_that_is_not_there_is_an_error() {
        let mut console = console(FakeCard::default());
        assert_eq!(talk(&mut console, &["@@ 1 ls sounds".into()]), ["@@ 1 error no such directory"]);
    }

    #[test]
    fn a_file_is_copied_on_the_card_into_the_devices_directory_only() {
        let card = FakeCard::with("sounds/alarm/Zen.mp3", b"ID3");
        let mut console = console(card.clone());
        let replies = talk(
            &mut console,
            &[
                "@@ 1 cp sounds/alarm/Zen.mp3\tcute-display/apps/alarm/ringtones/Zen.mp3".into(),
                "@@ 2 cp sounds/alarm/Zen.mp3\tsounds/Zen.mp3".into(),
                "@@ 3 cp sounds/alarm/None.mp3\tcute-display/None.mp3".into(),
            ],
        );
        assert_eq!(replies[0], "@@ 1 ok");
        assert_eq!(card.file("cute-display/apps/alarm/ringtones/Zen.mp3").as_deref(), Some(b"ID3".as_slice()));
        assert!(replies[1].starts_with("@@ 2 error"));
        assert_eq!(card.file("sounds/Zen.mp3"), None);
        assert_eq!(replies[2], "@@ 3 error no such file");
    }

    #[test]
    fn a_large_file_is_read_a_range_at_a_time() {
        let contents: Vec<u8> = (0..=255u8).cycle().take(GET_RANGE_BYTES * 2 + 100).collect();
        let mut console = console(FakeCard::with("sounds/Snow storm_loop.wav", &contents));
        let mut read = Vec::new();
        loop {
            let replies = talk(&mut console, &[format!("@@ 1 get {} sounds/Snow storm_loop.wav", read.len())]);
            let range = decode_get(&replies, "1");
            assert_eq!(replies.last().unwrap(), &format!("@@ 1 ok {} {}", range.len(), crc32fast::hash(&range)));
            read.extend_from_slice(&range);
            if range.len() < GET_RANGE_BYTES {
                break;
            }
        }
        assert_eq!(read, contents);
    }

    #[test]
    fn a_file_is_removed() {
        let card = FakeCard::with("cute-display/wifi.conf", b"x");
        let mut console = console(card.clone());
        assert_eq!(talk(&mut console, &["@@ 1 rm cute-display/wifi.conf".into()]), ["@@ 1 ok"]);
        assert_eq!(card.file("cute-display/wifi.conf"), None);
    }

    #[test]
    fn a_put_too_large_is_refused_before_any_data() {
        let mut console = console(FakeCard::default());
        let replies = talk(&mut console, &[format!("@@ 1 put {} 0 cute-display/big", MAX_PUT_BYTES + 1)]);
        assert!(replies[0].starts_with("@@ 1 error"));
    }

    #[test]
    fn the_log_between_the_lines_is_ignored() {
        let card = FakeCard::default();
        let mut console = console(card.clone());
        let mut lines = put_lines("1", "cute-display/a.conf", b"hello world", crc32fast::hash(b"hello world"));
        lines.insert(2, "I (1787) app: Cute Display 2026.9.0, build 09-25".into());
        assert_eq!(talk(&mut console, &lines), ["@@ 1 ok"]);
    }

    #[test]
    fn a_new_request_abandons_a_put_in_progress() {
        let card = FakeCard::default();
        let mut console = console(card.clone());
        let mut lines = put_lines("1", "cute-display/a.conf", b"hello", crc32fast::hash(b"hello"));
        lines.insert(1, "@@ 2 ls".into());
        let replies = talk(&mut console, &lines);
        assert_eq!(replies.last().unwrap(), "@@ 1 error no put in progress");
        assert_eq!(card.file("cute-display/a.conf"), None);
    }

    #[test]
    fn the_remote_taps_holds_and_turns() {
        let remote = Remote::new(NoWait);
        let mut console = MaintenanceConsole::new(Ok(FakeCard::default()), remote.clone(), Observation::default(), Stopped(MORNING));
        let mut yellow = remote.button(ButtonName::Yellow);
        let mut long = remote.button(ButtonName::Long);
        let mut wheel = remote.wheel();
        let lines = ["@@ 1 tap yellow", "@@ 2 hold 1500 yellow long", "@@ 3 turn -2"].map(String::from);
        assert_eq!(talk(&mut console, &lines), ["@@ 1 ok", "@@ 2 ok", "@@ 3 ok"]);
        assert_eq!((yellow.take_presses(), long.take_presses(), wheel.take_detents()), (2, 1, -2));
        assert!(!yellow.is_held() && !long.is_held(), "released once the hold is over");
    }

    #[test]
    fn without_a_card_the_remote_still_answers() {
        let mut console = MaintenanceConsole::<FakeCard>::new(Err(Fault::new("not inserted")), Remote::new(NoWait), Observation::default(), Stopped(MORNING));
        let replies = talk(&mut console, &["@@ 1 ls".into(), "@@ 2 turn 2".into()]);
        assert_eq!(replies, ["@@ 1 error no SD card: not inserted", "@@ 2 ok"]);
    }

    struct Lamp(Brightness);
    impl DimmableLight for Lamp {
        fn set_brightness(&mut self, brightness: Brightness) -> Result<(), Fault> {
            self.0 = brightness;
            Ok(())
        }
        fn brightness(&self) -> Brightness {
            self.0
        }
    }

    #[test]
    fn what_is_observed_is_answered() {
        let observation = Observation::default();
        let mut lamp = observation.light(LightName::ReadingLamp, Lamp(Brightness::OFF));
        lamp.set_brightness(Brightness::percent(30)).unwrap();
        let mut console = MaintenanceConsole::new(Ok(FakeCard::default()), Remote::new(NoWait), observation, Stopped(MORNING));
        let replies = talk(&mut console, &["@@ 1 lights".into(), "@@ 2 sound".into()]);
        assert_eq!(replies, ["@@ 1 ok 0 30", "@@ 2 ok silent"]);
    }

    struct Unreadable;
    impl RealTimeClock for Unreadable {
        fn read(&mut self) -> Result<ClockReading, Fault> {
            Err(Fault::new("I2C: no answer"))
        }
        fn set(&mut self, _: DateTime) -> Result<(), Fault> {
            Err(Fault::new("I2C: no answer"))
        }
    }

    #[test]
    fn the_clock_is_read_from_the_rtc_and_its_fault_said() {
        let mut console = console(FakeCard::default());
        assert_eq!(talk(&mut console, &["@@ 1 clock".into()]), [format!("@@ 1 ok {}", MORNING.unix_seconds())]);
        let mut console = MaintenanceConsole::new(Ok(FakeCard::default()), Remote::new(NoWait), Observation::default(), Unreadable);
        assert_eq!(talk(&mut console, &["@@ 1 clock".into(), "@@ 2 clock set 0".into()]), ["@@ 1 error I2C: no answer", "@@ 2 error I2C: no answer"]);
    }

    #[test]
    fn the_clock_set_is_the_clock_read() {
        let mut console = console(FakeCard::default());
        let replies = talk(&mut console, &["@@ 1 clock set 1790407815".into(), "@@ 2 clock".into()]);
        assert_eq!(replies, ["@@ 1 ok", "@@ 2 ok 1790407815"]);
    }

    struct Glass;
    impl EpaperDisplay for Glass {
        fn show(&mut self, _: &Frame, _: Redraw) -> Result<Refreshed, Fault> {
            Ok(Refreshed::Nothing)
        }
    }

    #[test]
    fn the_screen_comes_whole_with_the_times_it_was_shown() {
        let observation = Observation::default();
        let last_frame: Box<[u8; FRAME_BYTES]> = vec![0; FRAME_BYTES].into_boxed_slice().try_into().unwrap();
        let mut panel = observation.panel(Glass, last_frame);
        let mut console = MaintenanceConsole::new(Ok(FakeCard::default()), Remote::new(NoWait), observation, Stopped(MORNING));
        assert_eq!(talk(&mut console, &["@@ 1 screen".into()]), ["@@ 1 error nothing shown yet"]);
        let mut frame = Frame::blank();
        frame.set_ink(0, 0, true);
        panel.show(&frame, Redraw::Changes).unwrap();
        let replies = talk(&mut console, &["@@ 2 screen".into()]);
        assert_eq!(decode_get(&replies, "2"), frame.as_bytes());
        assert_eq!(replies.last().unwrap(), "@@ 2 ok 1");
    }
}
