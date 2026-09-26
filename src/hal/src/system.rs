pub trait SystemMonitor {
    fn free_heap_bytes(&self) -> u32;
}
