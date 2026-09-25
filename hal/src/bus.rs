pub trait I2cBus {
    /// 7-bit addresses of every device that acknowledges.
    fn scan(&mut self) -> Vec<u8>;
}
