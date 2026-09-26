use esp_idf_svc::sys::{esp_get_free_heap_size, heap_caps_get_largest_free_block, MALLOC_CAP_8BIT};
use hal::system::SystemMonitor;

pub struct EspSystem;

impl EspSystem {
    /// What the largest single allocation could be: TLS fails on this, not on the total.
    pub fn largest_free_block(&self) -> usize {
        // SAFETY: reads the allocator's counters, nothing else.
        unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_8BIT) }
    }
}

impl SystemMonitor for EspSystem {
    fn free_heap_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        unsafe { esp_get_free_heap_size() }
    }
}
