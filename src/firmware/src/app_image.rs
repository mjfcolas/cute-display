//! The app image on the board, in the directory and on the network its binary gives it.

use std::convert::Infallible;
use std::io::{self, BufRead, LineWriter};
use std::marker::PhantomData;
use std::thread;
use std::time::Duration;

use app::{Devices, Hardware};
use drivers::button::Button;
use drivers::ds3231::Ds3231Clock;
use drivers::esp_system::EspSystem;
use drivers::esp_wifi::EspWifiRadio;
use drivers::heap_bytes::PsramBytes;
use drivers::i2s_speaker::I2sSpeaker;
use drivers::ledc_light::LedcLight;
use drivers::pcnt_encoder::PcntEncoder;
use drivers::sdmmc_card::SdmmcCard;
use drivers::std_steady::StdSteadyClock;
use drivers::uc8253::Uc8253;
use drivers::usb_console;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::task::thread::{MallocCap, ThreadSpawnConfiguration};
use hal::http::HttpClient;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::composite_input::{CompositeButton, CompositeWheel};
use infrastructure::own_directory::CardDirectory;
use infrastructure::shared_rtc::SharedRtc;
use maintenance::observation::{LightName, Observation, ObservedLight, ObservedPanel, ObservedSpeaker};
use maintenance::remote::{ButtonName, Remote, RemoteButton, RemoteWheel};
use maintenance::MaintenanceConsole;

use crate::board::Board;

const BUILD: &str = concat!("built ", env!("BUILD_TIME"));
/// An MP3 decoded as it plays goes 7 KB deep on x86-64 in release, 21 KB in debug; not
/// measured on the Xtensa, hence the margin. In PSRAM: internal RAM has no room left for it
/// once the Wi-Fi is up. The thread neither writes the flash nor runs while its cache is off.
const SPEAKER_STACK_BYTES: usize = 32 * 1024;

struct Habity<Http, Udp>(PhantomData<fn() -> (Http, Udp)>);

impl<Http: HttpClient + Send + 'static, Udp: UdpClient + Send + 'static> Hardware for Habity<Http, Udp> {
    type Panel = ObservedPanel<Uc8253>;
    type Wheel = CompositeWheel<PcntEncoder, RemoteWheel>;
    type Button = CompositeButton<Button, RemoteButton>;
    type Light = ObservedLight<LedcLight>;
    type Rtc = SharedRtc<Ds3231Clock>;
    type Speaker = ObservedSpeaker<I2sSpeaker>;
    type Card = SdmmcCard;
    type Wifi = EspWifiRadio;
    type Http = Http;
    type Udp = Udp;
    type System = EspSystem;
    type Steady = StdSteadyClock;

    const NETWORK_STACK_BYTES: usize = 24 * 1024;

    fn spawn_speaker(play: impl FnOnce() + Send + 'static) -> Result<(), Fault> {
        let in_psram = ThreadSpawnConfiguration {
            stack_size: SPEAKER_STACK_BYTES,
            stack_alloc_caps: MallocCap::Spiram | MallocCap::Cap8bit,
            ..Default::default()
        };
        in_psram.set().map_err(|e| Fault::new(format!("speaker thread: {e}")))?;
        let spawned = thread::Builder::new().name("speaker".into()).stack_size(SPEAKER_STACK_BYTES).spawn(play);
        ThreadSpawnConfiguration::default().set().map_err(|e| Fault::new(format!("speaker thread: {e}")))?;
        spawned.map(drop).map_err(Fault::new)
    }
}

pub fn start_logging(image_name: &str) {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // The monitor re-attaches after resetting the chip; anything logged before is lost.
    FreeRtos::delay_ms(1500);
    log::info!("{image_name} {}, {BUILD}", app::image::VERSION);
}

pub fn run<Http, Udp>(board: Board, own_directory: CardDirectory, https: Http, udp: Udp) -> Result<Infallible, Fault>
where
    Http: HttpClient + Send + 'static,
    Udp: UdpClient + Send + 'static,
{
    let remote = Remote::new(StdSteadyClock);
    let observation = Observation::default();
    let rtc = SharedRtc::new(board.clock);
    start_maintenance(board.sd_card.clone(), own_directory, remote.clone(), observation.clone(), rtc.clone())?;
    let devices = Devices::<Habity<Http, Udp>> {
        panel: observation.panel(board.panel, PsramBytes::zeroed()?),
        wheel: CompositeWheel::new(board.wheel, remote.wheel()),
        wheel_button: CompositeButton::new(board.wheel_button, remote.button(ButtonName::WheelButton)),
        yellow_button: CompositeButton::new(board.yellow_button, remote.button(ButtonName::Yellow)),
        long_button: CompositeButton::new(board.long_button, remote.button(ButtonName::Long)),
        front_light: observation.light(LightName::FrontLight, board.front_light),
        reading_lamp: observation.light(LightName::ReadingLamp, board.reading_lamp),
        rtc,
        speaker: observation.speaker(board.speaker),
        sd_card: board.sd_card,
        wifi: board.wifi,
        https,
        udp,
        system: board.system,
        steady: StdSteadyClock,
    };
    app::run(devices, own_directory, catalog::APPS, move |lines| observation.said(lines))
}

fn start_maintenance(
    card: Result<SdmmcCard, Fault>,
    own_directory: CardDirectory,
    remote: Remote,
    observation: Observation,
    rtc: SharedRtc<Ds3231Clock>,
) -> Result<(), Fault> {
    let output = usb_console::listen()?;
    thread::Builder::new()
        .name("maintenance".into())
        .stack_size(16 * 1024)
        .spawn(move || {
            let mut console = MaintenanceConsole::new(card, own_directory.name(), remote, observation, rtc);
            // A line a call, so that no other output lands in the middle of it.
            let mut output = LineWriter::new(output);
            let mut line = String::new();
            loop {
                line.clear();
                let replied = io::stdin().lock().read_line(&mut line).and_then(|_| {
                    output.get_mut().heard_from_the_computer();
                    console.on_line(&line, &mut output)
                });
                if let Err(e) = replied {
                    log::warn!("maintenance: {e}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        })
        .map_err(Fault::new)?;
    Ok(())
}
