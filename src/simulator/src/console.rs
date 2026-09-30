//! The maintenance console on a Unix socket instead of the USB cable, one computer at a
//! time.

use std::path::Path;

use hal::storage::FileStorage;
use hal::Fault;
use maintenance::MaintenanceConsole;

#[cfg(unix)]
pub fn listen<S: FileStorage + Send + 'static>(path: &Path, mut console: MaintenanceConsole<S>) -> Result<(), Fault> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixListener;
    use std::thread;

    let fault = |e: std::io::Error| Fault::new(format!("console on {}: {e}", path.display()));
    // A socket left by a simulator that stopped would refuse the bind.
    match std::fs::symlink_metadata(path) {
        Ok(found) if found.file_type().is_socket() => std::fs::remove_file(path).map_err(fault)?,
        Ok(_) => return Err(Fault::new(format!("console on {}: a file that is not a socket is there", path.display()))),
        Err(_) => {}
    }
    let listener = UnixListener::bind(path).map_err(fault)?;
    log::info!("console: on {}", path.display());
    thread::Builder::new()
        .name("maintenance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let served = stream.and_then(|stream| {
                    let mut out = stream.try_clone()?;
                    for line in BufReader::new(stream).lines() {
                        console.on_line(&line?, &mut out)?;
                    }
                    Ok(())
                });
                if let Err(e) = served {
                    log::warn!("console: {e}");
                }
            }
        })
        .map_err(Fault::new)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn listen<S: FileStorage + Send + 'static>(_: &Path, _: MaintenanceConsole<S>) -> Result<(), Fault> {
    Err(Fault::new("the console needs a Unix socket"))
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    use hal::input::PushButton;
    use hal_testing::steady::FakeSteadyClock;
    use maintenance::observation::Observation;
    use maintenance::remote::{ButtonName, Remote};

    use super::*;
    use crate::card::DirectoryCard;
    use crate::clock::HostClock;
    use crate::steady::ScaledClock;

    fn rtc() -> HostClock {
        HostClock::new(ScaledClock::new(core::num::NonZeroU32::MIN)).unwrap()
    }

    fn directory(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("cute-display-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn a_computer_taps_a_button_through_the_socket_even_where_one_was_left() {
        let root = directory("console-left");
        let path = root.join("console.sock");
        drop(UnixListener::bind(&path).unwrap());
        let remote = Remote::new(FakeSteadyClock::default());
        let mut yellow = remote.button(ButtonName::Yellow);
        listen(&path, MaintenanceConsole::new(DirectoryCard::open(&root), remote, Observation::default(), rtc())).unwrap();

        let mut stream = UnixStream::connect(&path).unwrap();
        stream.write_all(b"@@ 1 tap yellow\n").unwrap();
        let mut reply = String::new();
        BufReader::new(&stream).read_line(&mut reply).unwrap();
        assert_eq!(reply, "@@ 1 ok\n");
        assert_eq!(yellow.take_presses(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_file_that_is_not_a_socket_is_left_alone() {
        let root = directory("console-file");
        let path = root.join("settings.conf");
        std::fs::write(&path, b"backlight = 5s\n").unwrap();
        assert!(listen(&path, MaintenanceConsole::new(DirectoryCard::open(&root), Remote::new(FakeSteadyClock::default()), Observation::default(), rtc())).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"backlight = 5s\n");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
