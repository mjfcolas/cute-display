pub trait I2cBus {
    /// The 7-bit addresses of every device that acknowledges.
    fn scan(&mut self) -> Vec<u8>;
}
