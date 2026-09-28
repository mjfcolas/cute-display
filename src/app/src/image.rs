use core::ffi::c_char;

pub const PROJECT: &str = "cute-display";
pub const VERSION: &str = env!("CUTE_DISPLAY_VERSION");

/// In a const context, as the images' description uses it, a `text` that leaves no room
/// for the NUL does not build; called at run time, it would panic.
pub const fn c_text<const N: usize>(text: &str) -> [c_char; N] {
    let mut array = [0; N];
    assert!(text.len() < N, "the text does not fit the C array");
    let (mut rest, mut slots): (&[u8], &mut [c_char]) = (text.as_bytes(), &mut array);
    while let (Some((byte, bytes_left)), Some((slot, slots_left))) = (rest.split_first(), slots.split_first_mut()) {
        *slot = *byte as c_char;
        (rest, slots) = (bytes_left, slots_left);
    }
    array
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_copied_then_nul_padded() {
        let array: [c_char; 8] = c_text("2026.9");
        assert_eq!(array.map(|c| c as u8), *b"2026.9\0\0");
    }

    #[test]
    fn the_version_is_a_tag_or_says_it_is_untagged() {
        assert!(VERSION.chars().next().is_some_and(|c| c.is_ascii_digit()), "{VERSION}");
        let _: [c_char; 32] = c_text(VERSION);
    }
}
