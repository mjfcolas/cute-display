# Design

## Layers

```
                 ┌──────────┐
                 │ firmware │  composition roots: one binary per image
                 └────┬─────┘
       ┌──────────────┼──────────────────┬──────────────┐
       ▼              ▼                  ▼              ▼
   ┌──────┐   ┌────────────────┐    ┌─────────┐    ┌────────┐
   │  ui  │──▶│     domain     │◀───│ infra-  │───▶│  hal   │◀───┐
   └──────┘   │ concepts +     │    │structure│    │contracts│   │
              │ contracts      │    └─────────┘    └────────┘    │
              └────────────────┘                          ┌──────┴──┐
                                                          │ drivers │
                                                          └─────────┘
```

| Layer            | Holds                                                                                          | Depends on       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ---------------- |
| `domain`         | Every functional concept (alarm, schedule, settings, sound…) and the contracts it needs from the outside world. Speaks in intents ("snooze"), never in controls ("yellow pressed") | nothing          |
| `infrastructure` | Implementations of the domain's contracts: settings on the SD card, time from the RTC and NTP…  | `domain`, `hal`  |
| `hal`            | Contracts with the hardware: display, encoder, buttons, lights, speaker, clock, storage, radio, power | nothing          |
| `drivers`        | Implementations of `hal` on the board's chips                                                  | `hal`            |
| `ui`             | A screen per app, the system app's included: reads the controls, turns them into domain intents, renders domain state. Runs on its own thread | `domain`         |
| `firmware`       | Composition roots: builds the drivers from the pin map (`board`), wires the layers, runs them  | everything       |

The arrows are the only allowed dependencies. In particular the domain never sees a HAL
type, the UI never sees hardware, and nothing but `firmware` knows which chip is on
which pin.

The UI meets the hardware through two exchange surfaces, which `firmware` connects:

- **out**: embedded-graphics' `DrawTarget`, a library trait that `hal::display::Frame`
  implements and that every screen draws on;
- **in**: `ui::controls::ControlsSample`, plain data owned by the UI (detents, presses,
  buttons held), filled from the HAL's `RotaryEncoder` and `PushButton`.

## SOLID, applied

- **Single responsibility**: a crate per layer, a module per concept. A driver drives
  one chip; a screen shows one thing.
- **Open/closed**: new hardware is a new driver behind an existing contract; a new
  storage backend is a new infrastructure implementation. Neither changes its callers.
- **Liskov**: every implementation honours its contract's documented behaviour, which is
  what lets the host tests replace hardware with fakes.
- **Interface segregation**: contracts are small and named for one capability
  (`RotaryEncoder`, `PushButton`, `DimmableLight`), not one interface per board.
- **Dependency inversion**: the layer that needs something owns the contract
  (`domain` for the application, `hal` for the hardware); the implementing layer
  depends on it, never the reverse.

## Contracts

- HAL contracts are **blocking and single-owner**. Deciding what runs on which thread
  belongs to the layer that uses them.
- Failures are values: `hal::Fault` describes what the hardware did not do.
- Contracts speak in the product's units (`Brightness`, `Temperature`, `Redraw`), not
  in register values.

## Threads

A slow device gets a thread of its own, and talks to the rest through a channel in and
a "latest value" out:

| Thread   | Owns                   | Why                                               |
| -------- | ---------------------- | ------------------------------------------------- |
| main     | domain services (lighting) | they must keep running whatever is on screen  |
| ui       | controls, apps, display | a refresh blocks for 0.35 to 3 s                 |
| painter  | the e-paper display (hwtest) | a refresh blocks for 0.35 to 3 s            |
| chimes   | the speaker            | playback blocks until the sound ends              |
| survey   | the Wi-Fi radio        | a scan blocks for seconds                         |
| buttons  | the button pins        | presses must be counted while everyone else is busy |
| maintenance | the USB console's input | it waits for lines from a computer            |
| network  | the Wi-Fi and HTTPS client, for weather and radar | a fetch waits on the network for seconds; one thread, since each stack is heap TLS needs |

## Where things live

```
domain/     what the product does: the apps, the one in front, settings, lighting, weather,
            radar, places
infrastructure/ the domain's contracts on the HAL: conf files, lights, Internet on demand,
            Open-Meteo
ui/         app screens (the system app's among them), gestures; host-tested with hal's
            Frame as a dev-dependency
hal/        contracts with the hardware, and the Frame the display shows
drivers/    ESP32-S3 implementations of hal; chip logic that needs no ESP32 is host-tested
maintenance/ the console that serves the SD card on the USB cable
hwtest/     the hardware test bench (see below)
firmware/   board pin map + one binary per image (`app`, `hwtest`); the only crate built
            for the ESP32 only
tools/      host scripts (frame dump to PNG)
```

## Applications

The main image (`firmware/src/bin/app.rs`) is a set of small apps, one in front at a
time.

**The domain owns the apps.** `domain::apps::App` lists them and `Foreground` says which
one is in front; anything may bring an app forward, not only the person at the controls
(when an alarm rings, the alarm's own service will). Each app's state and rules are domain
concepts too (`Weather`, `Radar`), shared with whoever needs them.

**The system app is an app with a special status.** It lists the other apps and holds the
device's settings. It is not in the list (`App::LAUNCHABLE`), only the system gesture
reaches it, and leaving it goes back to the app it was opened from, which `Foreground`
remembers. Its settings are domain concepts:

- `domain::settings`: the backlight duration and the reading lamp's level (off, 10, 30,
  50, 100 %), kept by
  a `SettingsStore` on every change. `infrastructure::settings_file` keeps them as a few
  lines of text, `cute-display/settings.conf` on the SD card; without a card they last
  until the power goes.
- `domain::lighting`: the light over the screen comes up when the controls are touched
  and goes out once the backlight duration has passed; the reading lamp shines at its
  setting. The domain decides every level, the screen's 20 % included; lights are a
  `Light` contract taking a level, which `infrastructure::hal_light` implements on the
  HAL's `DimmableLight`. The main thread refreshes it ten times a second.

**The UI owns how they are seen and steered.**

- `ui::AppScreen`: one per app, the system app's included. Turns the controls into
  intents on the domain and draws what the domain says.
- `ui::apps::SystemScreen`: one list, the apps then the settings. The wheel moves the dot,
  which starts on the app the system app was opened from; a press opens an app or moves a
  setting to its next value; the long button goes back.
- `ui::gestures`: holding the long button for a second is the system's: it opens the
  system app, or closes it. A shorter press of the long button reaches the app, on
  release; every other control reaches it at once. Any control used also tells
  `Lighting` that the device was touched.
- `ui::Shell`: gives the app in front the whole glass, bar a thin margin (no title: the
  screen says what it is), and tells a screen when it comes to the front.

**Refreshing**: the UI thread redraws after any input; whenever the app in front is not
the one on the glass; and whenever the app in front says it changed on its own, by moving
its `AppScreen::version` on (the weather screen does, when a fetch lands and every minute
for its "updated … ago"). A new app in front is a whole, clean redraw; anything else only
redraws what changed. Controls keep counting during a refresh (PCNT, the buttons'
thread), and what piled up is handled in one go before the next refresh.

### Weather

The first real app: today's weather and the week's, at one place.

- `domain::weather`: `Sky`, `Forecast` (today, then the week), and `Weather`, which
  fetches once at start, every hour and on request, retries ten minutes after a failure,
  and keeps the last forecast through failures. It needs a `PlaceSource` and a
  `ForecastSource`; `domain::calendar` gives a date its weekday.
- `infrastructure::open_meteo` asks Open-Meteo (free, no key, dates in the place's own
  time zone, so no clock is needed to know which day it is), for the place in
  `cute-display/weather.conf`.
- `ui::apps::WeatherScreen`: the wheel turns to the week and back, a press asks for an
  update; the foot of the screen says how fresh the forecast is, or what is missing.
  `ui::weather_icons` draws each sky from shapes, at any size.

### Radar

The aircraft around a place, as a radar scope, with the nearest listed beside it.

- `domain::radar`: `Aircraft`, `Range` (5 to 100 km) and `Radar`, which fetches every
  15 s, **only while the radar app is in front**, and at once when the range changes. It
  keeps the 60 nearest aircraft.
- `infrastructure::adsb_fi` asks adsb.fi's open data (free, no key, personal and
  non-commercial use, credited on the screen), for the place in `cute-display/radar.conf`.
  A busy sky is tens of kilobytes of JSON, so the answer is read **as it arrives**
  (`HttpClient` hands over a stream) and only the nearest aircraft are ever held.
- `ui::apps::RadarScreen`: north up, the place as a small cross, rings at half and full
  range, airports as dots, a triangle per aircraft pointing along its track. Aircraft are
  called by the last two letters of their registration (`F-GKXA` is `XA`), on the scope
  and in front of their line in the list, so the two can be matched. The scope takes the
  whole height; the place, the range, the five nearest and, at the foot, any trouble, the
  controls and the source share the column beside it. The wheel sets the range, a press
  asks for an update.
- **Labels never cover each other** (`ui::radar_view::place_labels`): named airports
  first, then aircraft nearest first; each tries eight spots around its mark, beside
  before above and below before the corners, and takes the first that covers no label, no
  mark and nothing outside the scope. One with nowhere to go goes without.
- Airports come from `cute-display/airports.conf`, one a line (code, latitude, longitude,
  name), read by `infrastructure::airports_file` at every update. `tools/airports.py`
  (`just radar-airports`) writes it from OurAirports: every airport and airfield within
  100 km of the place in `radar.conf`, read off the device. Which ones are named is the
  radar's own setting, `airport_labels` in `radar.conf`, since `airports.conf` is written
  again whenever the place changes.

### Places and the Internet

- `domain::place`: a `GeoPoint`, its distance and east/north offset to another (a flat
  projection, well under a percent off at 100 km), and a named `Place`.
  `infrastructure::place_file::PlaceFile` reads one from a conf file (`place`, `latitude`,
  `longitude`); weather and radar each have theirs.
- `domain::fetch`: `Unavailable` (why the outside gave nothing, in words a person can act
  on) and `FetchStatus`, shared by every service that fetches.
- `infrastructure::internet`: `OnDemandInternet` joins the Wi-Fi in
  `cute-display/wifi.conf` on the first request and leaves it after a minute with none;
  a failed request leaves at once, to start afresh. `SharedInternet` hands the one
  connection to weather and radar, whose requests take turns. Conf files are read again
  at every fetch, so a file dropped with `just sd-put` needs no restart.
- `firmware`: the `network` thread runs both services' `refresh_if_due` every second, and
  releases the Wi-Fi once idle.

## Maintenance console

The SD card does not come out of the case, so the app serves it on the USB cable: a
`maintenance` thread reads the console line by line, and `tools/sd.py` (`just sd-ls`,
`sd-get`, `sd-put`, `sd-rm`) is the other end. It is how configuration prepared on a
computer (Wi-Fi, a place for a weather app…) reaches the device: a file dropped in
`cute-display/`, read by whoever needs it.

- **Everything may be read; only `cute-display/` may be written or removed.** The stock
  firmware's sounds are out of reach of a mistyped command. The device's own settings
  live there too, in `cute-display/settings.conf`.
- **The protocol** (`maintenance::protocol`): every line starts with `@@ <id>`, and both
  ends ignore every other line, since the device's log shares the console. `ls`, `get`
  and `rm` are one request and their replies; a `put` announces its size and CRC32, gets
  `ready`, then sends 48 bytes of base64 per `data` line and waits for each `ack`,
  because the console drops what arrives faster than it reads. Nothing is written until
  the whole file arrived with the right CRC.
- **In the layers**, it is a second way into the device beside the UI, working on files
  rather than on the domain: `maintenance` depends on `hal` alone, like `hwtest`.
- `drivers::usb_console` makes stdin wait for a line. Output never waits for a computer:
  with nobody reading, it is dropped after one 50 ms wait.

## The hardware test

A second image, the **hardware test bench** (`hwtest`), exercises the HAL directly and
reports what each part of the board says; it has no domain or UI layer on purpose. It is
the way to check a board.

- `hwtest::bench` — the loop: controls drive the lights, the speaker and the page;
  sensors are read every second.
- `hwtest::report` — what a check is, and how each reading becomes one.
- `hwtest::screen` — the report page and the checkerboard, drawn into a `Frame`.
- `firmware/src/bin/hwtest.rs` — builds the board, hands its devices to the bench.
