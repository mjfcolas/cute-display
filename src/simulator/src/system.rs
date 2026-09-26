use hal::system::SystemMonitor;

/// What the computer has left to give, from `/proc/meminfo`; nothing elsewhere than Linux.
pub struct HostSystem;

impl SystemMonitor for HostSystem {
    fn free_heap_bytes(&self) -> u32 {
        std::fs::read_to_string("/proc/meminfo").ok().and_then(|meminfo| available_bytes(&meminfo)).unwrap_or(0)
    }

    /// A computer's memory is not fragmented the way the ESP32's heap is.
    fn largest_free_block_bytes(&self) -> u32 {
        self.free_heap_bytes()
    }
}

fn available_bytes(meminfo: &str) -> Option<u32> {
    let kib: u64 = meminfo.lines().find_map(|line| line.strip_prefix("MemAvailable:"))?.trim().strip_suffix("kB")?.trim().parse().ok()?;
    Some(u32::try_from(kib.saturating_mul(1024)).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_memory_is_read_in_bytes_and_saturates() {
        assert_eq!(available_bytes("MemTotal: 9 kB\nMemAvailable:     2048 kB\n"), Some(2 * 1024 * 1024));
        assert_eq!(available_bytes("MemAvailable: 99999999 kB\n"), Some(u32::MAX));
        assert_eq!(available_bytes("MemTotal: 9 kB\n"), None);
    }
}
