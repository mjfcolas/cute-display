//! The console on the USB cable, set up so that reading stdin waits for a line instead of
//! returning nothing. Its output goes to the driver many bytes a call: stdout hands the
//! driver one byte at a time, which caps the cable at about 45 KB/s.

use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::sys::{esp, esp_vfs_usb_serial_jtag_use_driver, usb_serial_jtag_driver_config_t, usb_serial_jtag_driver_install, usb_serial_jtag_write_bytes, TickType_t};
use hal::Fault;

use crate::console_output::{ConsoleOutput, TxRing};
use crate::or_fault::OrFault;

const RX_BUFFER_BYTES: u32 = 1024;
const TX_BUFFER_BYTES: usize = 4096;
const TX_WAIT: TickType_t = TickType::new_millis(50).ticks();

pub fn listen() -> Result<ConsoleOutput<UsbSerialJtagTx>, Fault> {
    let mut config = usb_serial_jtag_driver_config_t { tx_buffer_size: TX_BUFFER_BYTES as u32, rx_buffer_size: RX_BUFFER_BYTES };
    // SAFETY: a valid configuration, and the driver is installed once, before any read.
    esp!(unsafe { usb_serial_jtag_driver_install(&mut config) }).or_fault("USB console driver")?;
    // SAFETY: switches stdin and stdout over to the driver just installed.
    unsafe { esp_vfs_usb_serial_jtag_use_driver() };
    Ok(ConsoleOutput::new(UsbSerialJtagTx(())))
}

/// Unlike stdout, it does not wait for a log line being written: a line of its own may
/// land in the middle of one.
pub struct UsbSerialJtagTx(());

impl TxRing for UsbSerialJtagTx {
    /// The ring buffer takes nothing larger than itself.
    const BYTES: usize = TX_BUFFER_BYTES;

    fn push(&mut self, bytes: &[u8], wait: bool) -> bool {
        // SAFETY: `bytes` is valid for its length, which the driver copies before returning.
        unsafe { usb_serial_jtag_write_bytes(bytes.as_ptr().cast(), bytes.len(), if wait { TX_WAIT } else { 0 }) > 0 }
    }
}
