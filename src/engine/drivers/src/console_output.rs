//! The console's output, which does not wait for a computer: with nobody reading, what
//! finds no room is dropped after one wait, and nothing waits again until a write goes
//! through or the computer writes. The queue still holds what came while nobody read: an
//! answer that did not wait for it to drain would be dropped.

use std::io;

/// The driver's queue of bytes to send.
pub trait TxRing {
    /// The most one push takes.
    const BYTES: usize;
    /// Whether `bytes` were taken whole, waiting for room or not.
    fn push(&mut self, bytes: &[u8], wait: bool) -> bool;
}

pub struct ConsoleOutput<R> {
    ring: R,
    nobody_reading: bool,
}

impl<R> ConsoleOutput<R> {
    pub fn new(ring: R) -> Self {
        Self { ring, nobody_reading: false }
    }

    pub fn heard_from_the_computer(&mut self) {
        self.nobody_reading = false;
    }
}

impl<R: TxRing> io::Write for ConsoleOutput<R> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(sent) = bytes.get(..bytes.len().min(R::BYTES)).filter(|sent| !sent.is_empty()) else {
            return Ok(0);
        };
        self.nobody_reading = !self.ring.push(sent, !self.nobody_reading);
        Ok(sent.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::Write;

    use super::*;

    /// Has room for a push or not, in the order a test sets, and room once they are over;
    /// keeps each push's length and whether it waited.
    #[derive(Default)]
    struct StubTxRing {
        room: VecDeque<bool>,
        pushes: Vec<(usize, bool)>,
    }

    impl TxRing for StubTxRing {
        const BYTES: usize = 4;

        fn push(&mut self, bytes: &[u8], wait: bool) -> bool {
            self.pushes.push((bytes.len(), wait));
            self.room.pop_front().unwrap_or(true)
        }
    }

    #[test]
    fn a_write_hands_the_ring_no_more_than_it_takes() {
        let mut output = ConsoleOutput::new(StubTxRing::default());
        output.write_all(b"@@ 1 ok\n").unwrap();
        assert_eq!(output.ring.pushes, [(4, true), (4, true)]);
    }

    #[test]
    fn once_a_wait_found_no_room_nothing_waits_until_a_push_goes_through() {
        let mut output = ConsoleOutput::new(StubTxRing { room: VecDeque::from([false, false, true]), ..StubTxRing::default() });
        for _ in 0..4 {
            output.write_all(b"@@").unwrap();
        }
        assert_eq!(output.ring.pushes, [(2, true), (2, false), (2, false), (2, true)]);
    }

    #[test]
    fn once_the_computer_writes_an_answer_waits_for_room_again() {
        let mut output = ConsoleOutput::new(StubTxRing { room: VecDeque::from([false]), ..StubTxRing::default() });
        output.write_all(b"log
").unwrap();
        output.heard_from_the_computer();
        output.write_all(b"@@
").unwrap();
        assert_eq!(output.ring.pushes, [(4, true), (3, true)]);
    }
}
