pub mod softcore;
pub mod virt;
pub mod xiangshan;

use core::fmt;

use crate::arch;
use crate::logger;

// Platform selection
#[cfg(any(test, feature = "softcore"))]
type CurrentPlatform = softcore::SoftcorePlatform;
#[cfg(all(not(any(test, feature = "softcore")), feature = "xiangshan"))]
type CurrentPlatform = xiangshan::XSPlatform;
#[cfg(not(any(test, feature = "softcore", feature = "xiangshan")))]
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
    arch::init();
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
