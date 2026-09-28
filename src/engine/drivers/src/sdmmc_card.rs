use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::sd::mmc::SdMmcHostDriver;
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{esp_vfs_fat_info, ESP_OK};
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::PathBuf;

use hal::storage::{CopyOutcome, Entry, FileStorage};
use hal::Fault;

use crate::chunked_copy::copy_in_chunks;
use crate::or_fault::OrFault;

const MOUNT_POINT: &str = "/sdcard";
const MOUNT_POINT_C: &core::ffi::CStr = c"/sdcard";
const MAX_OPEN_FILES: usize = 4;
/// A copy moves this much at a time, from PSRAM rather than from a thread's stack.
const COPY_BUFFER_BYTES: usize = 32 * 1024;

/// Usable from any thread: FatFs locks the volume itself.
#[derive(Clone)]
pub struct SdmmcCard(());

impl SdmmcCard {
    pub fn mount(host: SdMmcHostDriver<'static>) -> Result<Self, Fault> {
        let card = SdCardDriver::new_mmc(host, &SdCardConfiguration::new()).or_fault("SD card")?;
        let fat = Fatfs::new_sdcard(0, card).or_fault("FAT on the SD card")?;
        let mounted = MountedFatfs::mount(fat, MOUNT_POINT, MAX_OPEN_FILES).or_fault("mounting the SD card")?;
        // Dropping the mount unmounts the card.
        core::mem::forget(mounted);
        Ok(Self(()))
    }
}

fn on_card(path: &str) -> PathBuf {
    PathBuf::from(MOUNT_POINT).join(path)
}

impl FileStorage for SdmmcCard {
    fn entries(&self, dir: &str) -> Result<Option<Vec<Entry>>, Fault> {
        let entries = match std::fs::read_dir(on_card(dir)) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Fault::new(format!("listing {dir} on the SD card: {e}"))),
        };
        let entries = entries.flatten().map(|e| {
            let metadata = e.metadata().ok();
            Entry {
                name: e.file_name().to_string_lossy().into_owned(),
                size_bytes: metadata.as_ref().map_or(0, |m| m.len()),
                is_dir: metadata.is_some_and(|m| m.is_dir()),
            }
        });
        Ok(Some(entries.collect()))
    }

    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, Fault> {
        match std::fs::read(on_card(path)) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Fault::new(format!("reading {path} on the SD card: {e}"))),
        }
    }

    fn read_range(&self, path: &str, offset: u64, max_bytes: usize) -> Result<Option<Vec<u8>>, Fault> {
        let mut file = match std::fs::File::open(on_card(path)) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Fault::new(format!("reading {path} on the SD card: {e}"))),
        };
        let mut contents = Vec::with_capacity(max_bytes);
        file.seek(SeekFrom::Start(offset))
            .and_then(|_| file.take(max_bytes as u64).read_to_end(&mut contents))
            .map_err(|e| Fault::new(format!("reading {path} on the SD card: {e}")))?;
        Ok(Some(contents))
    }

    fn write(&self, path: &str, contents: &[u8]) -> Result<(), Fault> {
        let file = on_card(path);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).or_fault("creating a directory on the SD card")?;
        }
        std::fs::write(file, contents).or_fault("writing on the SD card")
    }

    fn copy(&self, from: &str, to: &str) -> Result<CopyOutcome, Fault> {
        let mut source = match std::fs::File::open(on_card(from)) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(CopyOutcome::NoSource),
            Err(e) => return Err(Fault::new(format!("reading {from} on the SD card: {e}"))),
        };
        let destination = on_card(to);
        if let Some(dir) = destination.parent() {
            std::fs::create_dir_all(dir).or_fault("creating a directory on the SD card")?;
        }
        let mut copy = std::fs::File::create(destination).or_fault("writing on the SD card")?;
        copy_in_chunks(&mut source, &mut copy, &mut vec![0; COPY_BUFFER_BYTES])
            .map_err(|e| Fault::new(format!("copying {from} to {to} on the SD card: {e}")))?;
        Ok(CopyOutcome::Copied)
    }

    fn remove(&self, path: &str) -> Result<(), Fault> {
        match std::fs::remove_file(on_card(path)) {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(Fault::new(format!("removing {path} from the SD card: {e}"))),
            _ => Ok(()),
        }
    }

    fn capacity_bytes(&self) -> Result<u64, Fault> {
        let (mut total, mut free) = (0, 0);
        // SAFETY: a NUL-terminated path and two valid out-pointers.
        match unsafe { esp_vfs_fat_info(MOUNT_POINT_C.as_ptr(), &mut total, &mut free) } {
            ESP_OK => Ok(total),
            code => Err(Fault::new(format!("SD card capacity: error {code}"))),
        }
    }
}
