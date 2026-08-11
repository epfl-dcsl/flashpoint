//! QEMU Virt board

use core::fmt;
use core::fmt::Write;

use spin::Mutex;
use uart_16550::MmioSerialPort;

use super::Platform;

// —————————————————————————— Platform parameters ——————————————————————————— //

pub const SERIAL_PORT_BASE_ADDRESS: usize = 0x10000000;
const TEST_MMIO_ADDRESS: usize = 0x100000;

pub const TPM_TIS_BASE_ADDRESS: usize = 0x4000000;
pub const TPM_TIS_SIZE: usize = 0x5000;

static SERIAL_PORT: Mutex<Option<MmioSerialPort>> = Mutex::new(None);

// ———————————————————————————————— Platform ———————————————————————————————— //

#[allow(unused)]
pub struct VirtPlatform {}

impl Platform for VirtPlatform {
    #[allow(unused)]
    fn init() {
        // Serial
        let mut uart = SERIAL_PORT.lock();
        let mut mmio = unsafe { MmioSerialPort::new(SERIAL_PORT_BASE_ADDRESS) };
        mmio.init();
        *uart = Some(mmio);
    }

    fn debug_print(args: fmt::Arguments) {
        let mut serial_port = SERIAL_PORT.lock();
        if let Some(ref mut serial_port) = serial_port.as_mut() {
            serial_port
                .write_fmt(args)
                .expect("Printing to serial failed")
        };
        // drop(serial_port);
    }

    fn exit_success() -> ! {
        exit_qemu(true)
    }

    fn exit_failure() -> ! {
        exit_qemu(false)
    }
}

#[allow(unused)]
fn exit_qemu(success: bool) -> ! {
    let code = if success { 0x5555 } else { (1 << 16) | 0x3333 };

    unsafe {
        core::ptr::write_volatile(TEST_MMIO_ADDRESS as *mut i32, code);
    }

    // Loop forever if shutdown failed
    loop {
        core::hint::spin_loop();
    }
}
