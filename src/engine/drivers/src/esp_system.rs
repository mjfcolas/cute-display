use esp_idf_svc::sys::{heap_caps_get_free_size, heap_caps_get_largest_free_block, MALLOC_CAP_INTERNAL, MALLOC_CAP_SPIRAM};
use hal::system::SystemMonitor;

pub struct EspSystem;

impl SystemMonitor for EspSystem {
    fn free_heap_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        let bytes = unsafe { heap_caps_get_free_size(MALLOC_CAP_INTERNAL) };
        u32::try_from(bytes).unwrap_or(u32::MAX)
    }

    fn largest_free_block_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        let bytes = unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_INTERNAL) };
        u32::try_from(bytes).unwrap_or(u32::MAX)
    }

    fn free_psram_bytes(&self) -> Option<u32> {
        // SAFETY: reads the allocator's counters, nothing else.
        let bytes = unsafe { heap_caps_get_free_size(MALLOC_CAP_SPIRAM) };
        Some(u32::try_from(bytes).unwrap_or(u32::MAX))
    }
}
