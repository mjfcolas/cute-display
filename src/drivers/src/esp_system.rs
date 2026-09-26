use esp_idf_svc::sys::{esp_get_free_heap_size, heap_caps_get_largest_free_block, MALLOC_CAP_8BIT};
use hal::system::SystemMonitor;

pub struct EspSystem;

impl SystemMonitor for EspSystem {
    fn free_heap_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        unsafe { esp_get_free_heap_size() }
    }

    fn largest_free_block_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        let bytes = unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_8BIT) };
        u32::try_from(bytes).unwrap_or(u32::MAX)
    }
}
