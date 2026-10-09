//! Bytes where the heap would not put them by itself. In PSRAM: what is large and kept for
//! a long time, because stacks and DMA need the internal RAM. In internal RAM: what a
//! peripheral reaches by DMA, because not every peripheral can reach PSRAM by DMA.

use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use esp_idf_svc::sys::{heap_caps_calloc, heap_caps_free, MALLOC_CAP_DMA, MALLOC_CAP_INTERNAL, MALLOC_CAP_SPIRAM};
use hal::Fault;

/// `N` bytes in the memory that `CAPS` names, freed there.
pub struct HeapBytes<const N: usize, const CAPS: u32>(NonNull<[u8; N]>);

pub type PsramBytes<const N: usize> = HeapBytes<N, MALLOC_CAP_SPIRAM>;
pub type DmaBytes<const N: usize> = HeapBytes<N, { MALLOC_CAP_DMA | MALLOC_CAP_INTERNAL }>;

// SAFETY: only this value owns the bytes, and they are reached only through `&self` or
// `&mut self`.
unsafe impl<const N: usize, const CAPS: u32> Send for HeapBytes<N, CAPS> {}

impl<const N: usize, const CAPS: u32> HeapBytes<N, CAPS> {
    pub fn zeroed() -> Result<Self, Fault> {
        // SAFETY: `N` zeroed bytes, or null; bytes need no alignment beyond one.
        let bytes = unsafe { heap_caps_calloc(1, N, CAPS) }.cast::<[u8; N]>();
        let memory = if CAPS & MALLOC_CAP_SPIRAM != 0 { "PSRAM" } else { "internal RAM" };
        NonNull::new(bytes).map(Self).ok_or_else(|| Fault::new(format!("no {N} bytes left in {memory}")))
    }
}

impl<const N: usize, const CAPS: u32> Deref for HeapBytes<N, CAPS> {
    type Target = [u8; N];

    fn deref(&self) -> &[u8; N] {
        // SAFETY: allocated, zeroed, and owned by this value until it is dropped.
        unsafe { self.0.as_ref() }
    }
}

impl<const N: usize, const CAPS: u32> DerefMut for HeapBytes<N, CAPS> {
    fn deref_mut(&mut self) -> &mut [u8; N] {
        // SAFETY: allocated, zeroed, and owned by this value until it is dropped; `&mut
        // self` makes the access exclusive.
        unsafe { self.0.as_mut() }
    }
}

impl<const N: usize, const CAPS: u32> Drop for HeapBytes<N, CAPS> {
    fn drop(&mut self) {
        // SAFETY: from `heap_caps_calloc`, freed once.
        unsafe { heap_caps_free(self.0.as_ptr().cast()) }
    }
}
