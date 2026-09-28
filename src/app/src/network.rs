use std::thread;
use std::time::{Duration, Instant};

use domain::clock::Clock;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::system::SystemMonitor;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::internet::SharedInternet;

use crate::Service;

const NETWORK_PERIOD: Duration = Duration::from_secs(1);

/// One thread for everything that fetches: the requests share one Wi-Fi and take turns
/// anyway, and each thread's stack is internal RAM.
pub(crate) fn start<W, H, U, S, M>(
    clock: Clock,
    services: Vec<Service>,
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
        let psram = system.free_psram_bytes().map(|bytes| format!(", PSRAM {} KiB free", bytes / 1024)).unwrap_or_default();
        log::info!(
            "{what}: updating, internal {} KiB free, largest block {} KiB{psram}",
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
            for (app, service) in &services {
                if service.fetch_due(now) {
                    log_heap(app.name());
                    service.fetch(now);
                }
            }
            if let Some(internet) = &internet {
                internet.release_if_idle(Instant::now());
            }
            thread::sleep(NETWORK_PERIOD);
        })
        .map_err(Fault::new)?;
    Ok(())
}
