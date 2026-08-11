//! A dummy platform used when running with softcore-asm

use core::fmt;

use super::Platform;

// ———————————————————————————————— Platform ———————————————————————————————— //

#[allow(unused)]
pub struct SoftcorePlatform {}

impl Platform for SoftcorePlatform {
    fn init() {}

    fn debug_print(_args: fmt::Arguments) {
        // We only include std when running with softcore
        #[cfg(any(test, feature = "softcore"))]
        print!("{_args}");
    }

    fn exit_success() -> ! {
        // We only include std when running with softcore
        #[cfg(any(test, feature = "softcore"))]
        std::process::exit(0);

        // We need to add an extra unreachable when compiling on another target than softcore
        #[allow(unreachable_code)]
        {
            unreachable!()
        }
    }

    fn exit_failure() -> ! {
        panic!("Exit with failure")
    }
}
