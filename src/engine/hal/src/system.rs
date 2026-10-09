/// The memory that runs out first: on the device, the internal RAM that stacks and DMA
/// need.
pub trait SystemMonitor {
    fn free_heap_bytes(&self) -> u32;
    /// The size of the largest single allocation that can succeed: allocations fail on
    /// this, not on the total.
    fn largest_free_block_bytes(&self) -> u32;
    fn free_psram_bytes(&self) -> Option<u32> {
        None
    }
}
