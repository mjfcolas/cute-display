use domain::apps::{AppId, Foreground, Services};
use domain::clock::Clock;
use domain::files::Files;
use domain::internet::Internet;
use domain::place::PlaceSource;
use domain::sound::Sound;
use hal::http::HttpClient;
use hal::radio::WifiStation;
use hal::storage::FileStorage;
use hal::udp::UdpClient;
use hal::Fault;
use infrastructure::card_files::{CardFiles, UnreachableCard};
use infrastructure::general_file::{GeneralFile, NoGeneralFile};
use infrastructure::internet::{NoInternet, SharedInternet};
use infrastructure::speaker_sound::SpeakerSound;

/// When the SD card could not be reached, apps are told so by their files, and have no
/// Internet: the Wi-Fi's credentials are on the card.
pub(crate) struct EngineServices<W, H, U, S> {
    pub foreground: Foreground,
    pub clock: Clock,
    pub card: Result<S, Fault>,
    pub internet: Option<SharedInternet<W, H, U, S>>,
    pub sound: SpeakerSound,
}

impl<W, H, U, S> EngineServices<W, H, U, S> {
    pub fn to(&self, app: AppId) -> AppServices<'_, W, H, U, S> {
        AppServices { engine: self, app }
    }
}

pub(crate) struct AppServices<'a, W, H, U, S> {
    engine: &'a EngineServices<W, H, U, S>,
    app: AppId,
}

impl<W, H, U, S> Services for AppServices<'_, W, H, U, S>
where
    W: WifiStation + Send + 'static,
    H: HttpClient + Send + 'static,
    U: UdpClient + Send + 'static,
    S: FileStorage + Clone + Send + 'static,
{
    fn foreground(&self) -> Foreground {
        self.engine.foreground.clone()
    }

    fn clock(&self) -> Clock {
        self.engine.clock.clone()
    }

    fn internet(&self) -> Box<dyn Internet> {
        internet_through(self.engine.internet.as_ref())
    }

    fn place(&self) -> Box<dyn PlaceSource> {
        match &self.engine.card {
            Ok(card) => Box::new(GeneralFile::new(card.clone())),
            Err(_) => Box::new(NoGeneralFile),
        }
    }

    fn files(&self) -> Box<dyn Files> {
        files_on(&self.engine.card, self.app)
    }

    fn sound(&self) -> Box<dyn Sound> {
        Box::new(self.engine.sound.clone())
    }
}

fn internet_through<I: Internet + Clone + 'static>(internet: Option<&I>) -> Box<dyn Internet> {
    match internet {
        Some(internet) => Box::new(internet.clone()),
        None => Box::new(NoInternet("no SD card")),
    }
}

fn files_on<S: FileStorage + Clone + Send + 'static>(card: &Result<S, Fault>, app: AppId) -> Box<dyn Files> {
    match card {
        Ok(card) => Box::new(CardFiles::new(card.clone(), app)),
        Err(fault) => Box::new(UnreachableCard(fault.clone())),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use domain::fetch::Unavailable;
    use domain::internet::BodyReader;
    use hal::storage::Entry;

    use super::*;

    #[derive(Clone, Default)]
    struct Card(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

    impl FileStorage for Card {
        fn entries(&self, _: &str) -> Result<Option<Vec<Entry>>, Fault> {
            Ok(None)
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

    #[derive(Clone)]
    struct Online;

    impl Internet for Online {
        fn fetch(&mut self, _: &str, _: &mut BodyReader<'_>) -> Result<(), Unavailable> {
            Ok(())
        }
    }

    #[test]
    fn on_a_card_an_app_keeps_its_files_in_its_own_directory() {
        let card = Card::default();
        let files = files_on(&Ok(card.clone()), AppId::new("alarm"));
        files.write("alarm.conf", "enabled = yes").unwrap();
        assert!(card.0.lock().unwrap().contains_key("cute-display/apps/alarm/alarm.conf"));
    }

    #[test]
    fn without_the_card_files_say_so_and_there_is_no_internet() {
        let files = files_on(&Err::<Card, _>(Fault::new("not mounted")), AppId::new("alarm"));
        assert!(files.read("alarm.conf").is_err());
        assert!(internet_through(None::<&Online>).get("https://example.com").is_err());
        assert!(internet_through(Some(&Online)).get("https://example.com").is_ok());
    }
}
