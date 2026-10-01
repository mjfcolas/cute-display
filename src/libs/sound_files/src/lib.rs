mod chime;
mod mp3;
mod recording;
mod shared_files;
mod volume;
mod wav;

use domain::fetch::Unavailable;
use domain::files::Files;

pub use crate::chime::Chime;
pub use crate::recording::Recording;
pub use crate::shared_files::SharedFiles;
pub use crate::volume::VolumeKnob;

/// Reading the card stops the samples meanwhile, and the speaker holds about 30 ms of them:
/// 16 KB took 16 ms on the device, 4 KB leave room for decoding a frame beside.
const READ_BYTES: usize = 4 * 1024;
const EXTENSIONS: [&str; 2] = [".mp3", ".wav"];

/// The sound files in one of an app's directories, by name, in order.
pub fn sound_files_in(files: &dyn Files, directory: &str) -> Result<Vec<String>, Unavailable> {
    let names = files.names_in(directory)?;
    Ok(names.into_iter().filter(|name| EXTENSIONS.iter().any(|extension| name.to_lowercase().ends_with(extension))).collect())
}

/// A sound file's name as people read it: without its extension.
pub fn title(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use domain_testing::files::FakeFiles;

    use super::*;

    #[test]
    fn the_sound_files_are_the_mp3_and_wav_files_in_their_directory() {
        let files = FakeFiles::default();
        for name in ["sounds/Zen.mp3", "sounds/Default.MP3", "sounds/Rain.wav", "sounds/notes.txt", "sounds/old/Bell.mp3", "zen.conf"] {
            files.put(name, b"");
        }
        assert_eq!(sound_files_in(&files, "sounds"), Ok(vec!["Default.MP3".into(), "Rain.wav".into(), "Zen.mp3".into()]));
    }

    #[test]
    fn a_sound_file_is_titled_without_its_extension() {
        assert_eq!(title("Lost Ark.mp3"), "Lost Ark");
        assert_eq!(title("Rain"), "Rain");
    }
}
