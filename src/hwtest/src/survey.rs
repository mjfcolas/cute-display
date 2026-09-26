use std::thread;
use std::time::Duration;

use hal::radio::{AccessPoint, WifiScanner};
use hal::Fault;

use crate::latest::Latest;

const PERIOD: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Survey {
    Scanning,
    Failed(Fault),
    Found(Vec<AccessPoint>),
}

/// Scans for Wi-Fi networks every minute on a thread of its own, since a scan blocks
/// for seconds.
pub(crate) fn spawn(mut scanner: impl WifiScanner + Send + 'static) -> Result<Latest<Survey>, Fault> {
    let latest = Latest::new(Survey::Scanning);
    let published = latest.clone();
    thread::Builder::new()
        .name("survey".into())
        .stack_size(12 * 1024)
        .spawn(move || loop {
            let survey = match scanner.scan() {
                Ok(networks) => {
                    for n in &networks {
                        log::info!("wifi: {:>4} dBm ch{:<2} {}", n.rssi_dbm, n.channel, n.ssid);
                    }
                    Survey::Found(networks)
                }
                Err(fault) => Survey::Failed(fault),
            };
            published.set(survey);
            thread::sleep(PERIOD);
        })
        .map_err(Fault::new)?;
    Ok(latest)
}
