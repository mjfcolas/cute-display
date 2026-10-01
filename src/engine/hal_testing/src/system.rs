use hal::system::SystemMonitor;

/// Plenty of memory, never less.
pub struct StubSystemMonitor;

impl SystemMonitor for StubSystemMonitor {
    fn free_heap_bytes(&self) -> u32 {
        200 * 1024
    }

    fn largest_free_block_bytes(&self) -> u32 {
        100 * 1024
    }
}
