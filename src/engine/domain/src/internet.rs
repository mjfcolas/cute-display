use std::io::Read;

use crate::fetch::Unavailable;

/// The largest answer that a caller can hold whole with `get`. Anything larger must be
/// streamed.
pub const MAX_HELD_BYTES: u64 = 32 * 1024;

pub type BodyReader<'a> = dyn FnMut(&mut dyn Read) -> Result<(), Unavailable> + 'a;

pub trait Internet: Send {
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable>;

    fn get(&mut self, url: &str) -> Result<Vec<u8>, Unavailable> {
        let mut body = Vec::new();
        self.fetch(url, &mut |reader| {
            reader.take(MAX_HELD_BYTES + 1).read_to_end(&mut body).map_err(|e| Unavailable(e.to_string()))?;
            if body.len() as u64 > MAX_HELD_BYTES {
                return Err(Unavailable(format!("the answer is larger than {MAX_HELD_BYTES} bytes")));
            }
            Ok(())
        })?;
        Ok(body)
    }
}

impl<I: Internet + ?Sized> Internet for Box<I> {
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable> {
        I::fetch(self, url, read)
    }
}
