//! The PSRAM, for what is large and kept long: internal RAM is what stacks and DMA need.

use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use esp_idf_svc::sys::{heap_caps_calloc, heap_caps_free, MALLOC_CAP_SPIRAM};
use hal::Fault;

/// `N` bytes in PSRAM, freed there.
pub struct PsramBytes<const N: usize>(NonNull<[u8; N]>);

// SAFETY: the bytes are ours alone, reached only through `&self` or `&mut self`.
unsafe impl<const N: usize> Send for PsramBytes<N> {}

impl<const N: usize> PsramBytes<N> {
    pub fn zeroed() -> Result<Self, Fault> {
        // SAFETY: `N` zeroed bytes or null; bytes need no alignment beyond one.
        let bytes = unsafe { heap_caps_calloc(1, N, MALLOC_CAP_SPIRAM) }.cast::<[u8; N]>();
        NonNull::new(bytes).map(Self).ok_or_else(|| Fault::new(format!("no {N} bytes left in PSRAM")))
    }
}

impl<const N: usize> Deref for PsramBytes<N> {
    type Target = [u8; N];

    fn deref(&self) -> &[u8; N] {
        // SAFETY: allocated, zeroed and ours until dropped.
        unsafe { self.0.as_ref() }
    }
}

impl<const N: usize> DerefMut for PsramBytes<N> {
    fn deref_mut(&mut self) -> &mut [u8; N] {
        // SAFETY: allocated, zeroed and ours until dropped; `&mut self` makes it exclusive.
        unsafe { self.0.as_mut() }
    }
}

impl<const N: usize> Drop for PsramBytes<N> {
    fn drop(&mut self) {
        // SAFETY: from `heap_caps_calloc`, freed once.
        unsafe { heap_caps_free(self.0.as_ptr().cast()) }
    }
}
