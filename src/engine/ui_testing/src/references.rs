//! Rendering references: a frame is compared with the PNG a person looked at and kept.
//!
//! `UPDATE_REFERENCES=1 cargo test` writes the frames drawn as the new references, to be
//! looked at before they are committed. A frame that differs from its reference fails
//! the test and is written beside it as `<name>.seen.png`.

use std::fs;
use std::path::Path;

use hal::display::{Frame, HEIGHT, WIDTH};

const UPDATE: &str = "UPDATE_REFERENCES";
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
const GREYSCALE: u8 = 0;
const ONE_BIT: u8 = 1;
const NO_FILTER: u8 = 0;
/// What a stored deflate block holds at most.
const STORED_BLOCK_BYTES: usize = 0xffff;

/// Compares `frame` with `references/<name>.png` in the crate at `crate_dir`, which a
/// test gives as `env!("CARGO_MANIFEST_DIR")`.
pub fn check(frame: &Frame, crate_dir: &str, name: &str) {
    let references = Path::new(crate_dir).join("references");
    let (reference, seen) = (references.join(format!("{name}.png")), references.join(format!("{name}.seen.png")));
    let png = png(frame);
    let _ = fs::remove_file(&seen);
    if std::env::var_os(UPDATE).is_some_and(|value| value == "1") {
        fs::create_dir_all(&references).expect("the references' directory is made");
        fs::write(&reference, &png).expect("the reference is written");
        return;
    }
    let kept = fs::read(&reference).ok();
    assert!(kept.is_some(), "no reference at {}: run the test with {UPDATE}=1, then look at it", reference.display());
    let same = kept.as_deref() == Some(png.as_slice());
    if !same {
        fs::write(&seen, &png).expect("what was seen is written");
    }
    assert!(same, "the frame differs from {}: see {}; if the change is meant, run with {UPDATE}=1", reference.display(), seen.display());
}

/// Ink is black, paper white, the whole frame, hidden columns included.
fn png(frame: &Frame) -> Vec<u8> {
    let row_bytes = usize::from(WIDTH) / 8;
    let mut rows = Vec::with_capacity((row_bytes + 1) * usize::from(HEIGHT));
    for row in frame.as_bytes().chunks(row_bytes) {
        rows.push(NO_FILTER);
        rows.extend(row.iter().map(|ink| !ink));
    }
    let mut header = Vec::new();
    header.extend(u32::from(WIDTH).to_be_bytes());
    header.extend(u32::from(HEIGHT).to_be_bytes());
    header.extend([ONE_BIT, GREYSCALE, 0, 0, 0]);
    let mut png = SIGNATURE.to_vec();
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib_stored(&rows));
    chunk(&mut png, b"IEND", &[]);
    png
}

fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend((data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend(kind);
    png.extend(data);
    let crc = crc32(png.get(start..).unwrap_or_default());
    png.extend(crc.to_be_bytes());
}

/// Uncompressed: the same frame always gives the same bytes, and nothing to depend on.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = if data.is_empty() { vec![data] } else { data.chunks(STORED_BLOCK_BYTES).collect() };
    let mut blocks = blocks.into_iter().peekable();
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        let length = block.len() as u16;
        out.extend(length.to_le_bytes());
        out.extend((!length).to_le_bytes());
        out.extend(block);
    }
    out.extend(adler32(data).to_be_bytes());
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksums_are_those_of_png_and_zlib() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn nothing_is_still_one_final_stored_block() {
        assert_eq!(zlib_stored(&[]), [0x78, 0x01, 0x01, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn a_frame_is_a_one_bit_png_of_its_whole_size() {
        let png = png(&Frame::blank());
        assert_eq!(png[..8], SIGNATURE);
        assert_eq!(png[12..16], *b"IHDR");
        assert_eq!(png[16..24], [0, 0, 1, 160, 0, 0, 0, 240]);
        assert_eq!(png[24..26], [ONE_BIT, GREYSCALE]);
        assert_eq!(png[png.len() - 8..], [b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82]);
    }

    #[test]
    fn ink_is_black() {
        let mut frame = Frame::blank();
        frame.set_ink(0, 0, true);
        let rows_start = 8 + 25 + 8 + 2 + 5;
        let png = png(&frame);
        assert_eq!(png[rows_start..rows_start + 3], [NO_FILTER, 0x7f, 0xff]);
    }
}
