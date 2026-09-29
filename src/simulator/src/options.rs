use core::num::NonZeroU32;
use std::path::PathBuf;

const DEFAULT_CARD: &str = "sim-sd";

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub card: PathBuf,
    /// How many times faster than the wall the app image runs.
    pub speed: NonZeroU32,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self { card: PathBuf::from(DEFAULT_CARD), speed: NonZeroU32::MIN };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--speed" => {
                    let speed = args.next().ok_or("--speed: how many times faster?")?;
                    options.speed = speed.parse().map_err(|_| format!("--speed {speed}: a whole number from 1"))?;
                }
                flag if flag.starts_with("--") => return Err(format!("{flag}: no such option")),
                card => options.card = PathBuf::from(card),
            }
        }
        Ok(options)
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
    fn without_arguments_the_card_is_sim_sd_at_the_walls_speed() {
        assert_eq!(parse(&[]), Ok(Options { card: PathBuf::from("sim-sd"), speed: speed(1) }));
    }

    #[test]
    fn the_card_and_the_speed_come_in_any_order() {
        assert_eq!(parse(&["--speed", "20", "backup/sd"]), Ok(Options { card: PathBuf::from("backup/sd"), speed: speed(20) }));
    }

    #[test]
    fn a_speed_that_is_not_a_positive_number_or_an_unknown_flag_is_refused() {
        for bad in [&["--speed"][..], &["--speed", "0"], &["--speed", "fast"], &["--fast"]] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }
}
