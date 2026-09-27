/// The memory that runs short first: on the device, the internal RAM that stacks and DMA
/// need.
pub trait SystemMonitor {
    fn free_heap_bytes(&self) -> u32;
    /// What the largest single allocation could be: they fail on this, not on the total.
    fn largest_free_block_bytes(&self) -> u32;
    /// `None` without PSRAM.
    fn free_psram_bytes(&self) -> Option<u32> {
        None
    }
}
