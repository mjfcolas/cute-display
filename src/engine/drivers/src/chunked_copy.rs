use std::io::{self, Read, Write};

/// Through `chunk`, so a file of any size is never held whole.
pub fn copy_in_chunks(source: &mut impl Read, destination: &mut impl Write, chunk: &mut [u8]) -> io::Result<()> {
    loop {
        let read = source.read(chunk)?;
        match chunk.get(..read) {
            Some([]) | None => return Ok(()),
            Some(bytes) => destination.write_all(bytes)?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copied(source: &[u8]) -> Vec<u8> {
        let mut destination = Vec::new();
        copy_in_chunks(&mut &source[..], &mut destination, &mut [0; 4]).unwrap();
        destination
    }

    #[test]
    fn a_source_of_any_length_arrives_whole() {
        assert_eq!(copied(b""), b"");
        assert_eq!(copied(b"ID3"), b"ID3");
        assert_eq!(copied(b"0123456789"), b"0123456789");
    }
}
