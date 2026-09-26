pub trait SystemMonitor {
    fn free_heap_bytes(&self) -> u32;
    /// What the largest single allocation could be: TLS fails on this, not on the total.
    fn largest_free_block_bytes(&self) -> u32;
}
