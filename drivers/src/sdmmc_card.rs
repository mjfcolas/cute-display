use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::sd::mmc::SdMmcHostDriver;
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{esp_vfs_fat_info, ESP_OK};
use hal::storage::FileStorage;
use hal::Fault;

use crate::or_fault::OrFault;

const MOUNT_POINT: &str = "/sdcard";
const MOUNT_POINT_C: &core::ffi::CStr = c"/sdcard";
const MAX_OPEN_FILES: usize = 4;

/// A FAT-formatted SD card, mounted for the life of the device.
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

impl FileStorage for SdmmcCard {
    fn root_entries(&self) -> Result<Vec<String>, Fault> {
        let entries = std::fs::read_dir(MOUNT_POINT).or_fault("listing the SD card")?;
        Ok(entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
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
