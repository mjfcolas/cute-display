use std::thread;
use std::time::{Duration, Instant};

use domain::clock::Clock;
use domain::radar::Radar;
use domain::weather::Weather;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::internet::SharedInternet;

const NETWORK_PERIOD: Duration = Duration::from_secs(1);

/// Everything that fetches, on one thread: the requests share one Wi-Fi and take turns
/// anyway, and each thread's stack is heap that TLS needs.
pub(crate) fn start<W, H, U, S, M>(
    clock: Clock,
    weather: Weather,
    radar: Radar,
    internet: Option<SharedInternet<W, H, U, S>>,
    system: M,
    stack_bytes: usize,
) -> Result<(), Fault>
where
    W: WifiStation + Send + 'static,
    H: HttpClient + Send + 'static,
    U: UdpClient + Send + 'static,
    S: FileStorage + Send + 'static,
    M: SystemMonitor + Send + 'static,
{
    let log_heap = move |what: &str| {
        log::info!(
            "{what}: updating, {} KiB free, largest block {} KiB",
            system.free_heap_bytes() / 1024,
            system.largest_free_block_bytes() / 1024
        );
    };
    thread::Builder::new()
        .name("network".into())
        .stack_size(stack_bytes)
        .spawn(move || loop {
            let now = Instant::now();
            if let Err(unavailable) = clock.sync_if_due(now) {
                log::warn!("clock: not set from the network: {unavailable}");
            }
            if weather.is_due(now) {
                log_heap("weather");
                weather.refresh_if_due(now);
            }
            if radar.is_due(now) {
                log_heap("radar");
                radar.refresh_if_due(now);
            }
            if let Some(internet) = &internet {
                internet.release_if_idle(Instant::now());
            }
            thread::sleep(NETWORK_PERIOD);
        })
        .map_err(Fault::new)?;
    Ok(())
}
