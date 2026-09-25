use esp_idf_svc::sys::esp_get_free_heap_size;
use hal::system::SystemMonitor;

pub struct EspSystem;

impl SystemMonitor for EspSystem {
    fn free_heap_bytes(&self) -> u32 {
        // SAFETY: reads the allocator's counters, nothing else.
        unsafe { esp_get_free_heap_size() }
    }
}
