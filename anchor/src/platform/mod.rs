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

#[cfg(all(not(any(test, feature = "softcore")), feature = "xiangshan"))]
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;
#[cfg(not(any(test, feature = "softcore", feature = "xiangshan")))]
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

pub trait Platform {
    fn init();
    fn debug_print(args: fmt::Arguments);
    fn exit_success() -> !;
    fn exit_failure() -> !;
}

pub fn init() {
    CurrentPlatform::init();

    #[cfg(all(not(test), not(feature = "softcore")))]
    logger::init(LOG_LEVEL);
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
