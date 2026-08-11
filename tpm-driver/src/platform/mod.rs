pub mod virt;
pub mod xiangshan;

use core::fmt;

use crate::arch::{Arch, Architecture};
use crate::logger;

// Platform selection
#[cfg(feature = "xiangshan")]
type CurrentPlatform = xiangshan::XSPlatform;
#[cfg(not(feature = "xiangshan"))]
type CurrentPlatform = virt::VirtPlatform;


pub trait Platform {
    fn init();
    fn debug_print(args: fmt::Arguments);
    fn exit_success() -> !;
    fn exit_failure() -> !;
}

pub fn init() {
    CurrentPlatform::init();
    logger::init(log::LevelFilter::Info);
    // Trap handler
    Arch::init();
    //log::info!("Done with platform init.");
}

pub fn debug_print(args: fmt::Arguments) {
    CurrentPlatform::debug_print(args);
}

pub fn exit_success() -> ! {
    CurrentPlatform::exit_success();
}

#[allow(unused)]
pub fn exit_failure() -> ! {
    CurrentPlatform::exit_failure();
}
