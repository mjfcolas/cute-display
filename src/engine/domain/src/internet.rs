//! The Internet, as the engine lends it to whoever fetches: one request at a time, the
//! Wi-Fi brought up for it.

use std::io::Read;

use crate::fetch::Unavailable;

/// What a caller may hold whole with `get`; anything larger has to be streamed.
pub const MAX_HELD_BYTES: u64 = 32 * 1024;

pub type BodyReader<'a> = dyn FnMut(&mut dyn Read) -> Result<(), Unavailable> + 'a;

pub trait Internet: Send {
    /// Hands the body to `read` as it arrives.
    fn fetch(&mut self, url: &str, read: &mut BodyReader<'_>) -> Result<(), Unavailable>;

    /// The whole body, for answers small enough to hold.
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
