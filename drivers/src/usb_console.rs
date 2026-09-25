//! The console on the USB cable, set up so that reading stdin waits for a line instead of
//! returning nothing. Output does not wait for a computer: with nobody reading, it is
//! dropped after one 50 ms wait.

use esp_idf_svc::sys::{esp, esp_vfs_usb_serial_jtag_use_driver, usb_serial_jtag_driver_config_t, usb_serial_jtag_driver_install};
use hal::Fault;

use crate::or_fault::OrFault;

const BUFFER_BYTES: u32 = 1024;

pub fn listen() -> Result<(), Fault> {
    let mut config = usb_serial_jtag_driver_config_t { tx_buffer_size: BUFFER_BYTES, rx_buffer_size: BUFFER_BYTES };
    // SAFETY: a valid configuration, and the driver is installed once, before any read.
    esp!(unsafe { usb_serial_jtag_driver_install(&mut config) }).or_fault("USB console driver")?;
    // SAFETY: switches stdin and stdout over to the driver just installed.
    unsafe { esp_vfs_usb_serial_jtag_use_driver() };
    Ok(())
}
