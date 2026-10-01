use core::num::NonZeroU32;
use std::path::PathBuf;

use hal::clock::DateTime;

const DEFAULT_CARD: &str = "sim-sd";

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub card: PathBuf,
    /// How many times faster than the wall the app image runs.
    pub speed: NonZeroU32,
    pub console_socket: Option<PathBuf>,
    pub window: Window,
    pub start: StartTime,
    pub network: Network,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Window {
    Shown,
    /// The controls are the console's alone.
    Headless,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StartTime {
    TheComputers,
    Given(DateTime),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Network {
    TheComputers,
    /// HTTP answered from the responses recorded in this directory.
    Recorded(PathBuf),
    /// The computer's, each HTTP response recorded in this directory.
    Recording(PathBuf),
    Offline,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            card: PathBuf::from(DEFAULT_CARD),
            speed: NonZeroU32::MIN,
            console_socket: None,
            window: Window::Shown,
            start: StartTime::TheComputers,
            network: Network::TheComputers,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--speed" => {
                    let speed = args.next().ok_or("--speed: how many times faster?")?;
                    options.speed = speed.parse().map_err(|_| format!("--speed {speed}: a whole number from 1"))?;
                }
                "--console" => options.console_socket = Some(PathBuf::from(args.next().ok_or("--console: the socket's path")?)),
                "--headless" => options.window = Window::Headless,
                "--time" => {
                    let time = args.next().ok_or("--time: Unix seconds")?;
                    options.start = StartTime::Given(DateTime::from_unix_seconds(time.parse().map_err(|_| format!("--time {time}: Unix seconds"))?));
                }
                "--web" => options.network = options.network.chosen_once(Network::Recorded(PathBuf::from(args.next().ok_or("--web: the recordings' directory")?)))?,
                "--record" => {
                    options.network = options.network.chosen_once(Network::Recording(PathBuf::from(args.next().ok_or("--record: the recordings' directory")?)))?;
                }
                "--offline" => options.network = options.network.chosen_once(Network::Offline)?,
                flag if flag.starts_with("--") => return Err(format!("{flag}: no such option")),
                card => options.card = PathBuf::from(card),
            }
        }
        if options.window == Window::Headless && options.console_socket.is_none() {
            return Err("--headless: without a window, the controls need --console".into());
        }
        Ok(options)
    }
}

impl Network {
    fn chosen_once(self, chosen: Self) -> Result<Self, String> {
        match self {
            Self::TheComputers => Ok(chosen),
            _ => Err("--web, --record and --offline: one at most".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speed(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|&a| a.to_owned()))
    }

    #[test]
    fn without_arguments_the_card_is_sim_sd_at_the_walls_speed_in_a_window_on_the_computers_time_and_network() {
        assert_eq!(
            parse(&[]),
            Ok(Options {
                card: PathBuf::from("sim-sd"),
                speed: speed(1),
                console_socket: None,
                window: Window::Shown,
                start: StartTime::TheComputers,
                network: Network::TheComputers
            })
        );
    }

    #[test]
    fn the_card_and_the_flags_come_in_any_order() {
        assert_eq!(
            parse(&["--speed", "20", "--headless", "backup/sd", "--console", "target/simulator.sock", "--time", "1790407815", "--web", "web"]),
            Ok(Options {
                card: PathBuf::from("backup/sd"),
                speed: speed(20),
                console_socket: Some(PathBuf::from("target/simulator.sock")),
                window: Window::Headless,
                start: StartTime::Given(DateTime::from_unix_seconds(1_790_407_815)),
                network: Network::Recorded(PathBuf::from("web")),
            })
        );
    }

    #[test]
    fn the_network_is_recorded_or_cut() {
        assert_eq!(parse(&["--record", "web"]).map(|options| options.network), Ok(Network::Recording(PathBuf::from("web"))));
        assert_eq!(parse(&["--offline"]).map(|options| options.network), Ok(Network::Offline));
    }

    #[test]
    fn a_bad_value_an_unknown_flag_or_flags_that_contradict_are_refused() {
        let bad: [&[&str]; 10] = [
            &["--speed"],
            &["--speed", "0"],
            &["--speed", "fast"],
            &["--fast"],
            &["--console"],
            &["--time", "noon"],
            &["--web"],
            &["--web", "web", "--offline"],
            &["--offline", "--record", "web"],
            &["--headless"],
        ];
        for args in bad {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }
}
