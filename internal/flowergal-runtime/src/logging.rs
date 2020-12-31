/**
 * gba crate has its own logging macros, but I wanted a little more info in it
 *  & compiling out lower-level statements in release
 */
use core::fmt::{Arguments, Write};
use gba::mgba::{MGBADebug, MGBADebugLevel};

// FIXME: load-bearing logs in some places due to optimization errors, can't disable some of these...
#[cfg(not(debug_assertions))]
pub const STATIC_MAX_LEVEL: MGBADebugLevel = MGBADebugLevel::Info;
#[cfg(debug_assertions)]
pub const STATIC_MAX_LEVEL: MGBADebugLevel = MGBADebugLevel::Debug;

pub const STATIC_LEVEL_NAMES: [&str; 5] = ["F", "E", "W", "I", "D"];

#[instruction_set(arm::t32)]
pub fn internal_write_log(mgba: &mut MGBADebug, args: Arguments) {
    if let Some(s) = args.as_str() {
        let _ = mgba.write_str(s);
    } else {
        let _ = mgba.write_fmt(args);
    }
}

#[macro_export]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $message:expr) => ({
        let lvl = $lvl;
        if lvl as u16 <= $crate::logging::STATIC_MAX_LEVEL as u16 {
            if let Some(mut mgba) = gba::mgba::MGBADebug::new() {
                $crate::logging::internal_write_log(&mut mgba, format_args!(
                    "[{}] ({}) {}",
                    $crate::logging::STATIC_LEVEL_NAMES[lvl as usize],
                    $target,
                    $message
                ));
                mgba.send(lvl);
            }
        }
    });
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        let lvl = $lvl;
        if lvl as u16 <= $crate::logging::STATIC_MAX_LEVEL as u16 {
            if let Some(mut mgba) = gba::mgba::MGBADebug::new() {
                $crate::logging::internal_write_log(&mut mgba, format_args!(
                    "[{}] ({}) ",
                    $crate::logging::STATIC_LEVEL_NAMES[lvl as usize],
                    $target
                ));
                $crate::logging::internal_write_log(&mut mgba, format_args!($($arg)+));
                mgba.send(lvl);
            }
        }
    });
    ($lvl:expr, $($arg:tt)+) => ($crate::log!(target: module_path!(), $lvl, $($arg)+))
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, gba::mgba::MGBADebugLevel::Debug, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::log!(gba::mgba::MGBADebugLevel::Debug, $($arg)+);
    )
}

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, gba::mgba::MGBADebugLevel::Info, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::log!(gba::mgba::MGBADebugLevel::Info, $($arg)+);
    )
}

#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, gba::mgba::MGBADebugLevel::Warning, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::log!(gba::mgba::MGBADebugLevel::Warning, $($arg)+);
    )
}

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, gba::mgba::MGBADebugLevel::Error, $($arg)+);
    );
    ($($arg:tt)+) => (
        $crate::log!(gba::mgba::MGBADebugLevel::Error, $($arg)+);
    )
}

#[macro_export]
macro_rules! fatal {
    (target: $target:expr, $($arg:tt)+) => (
        $crate::log!(target: $target, gba::mgba::MGBADebugLevel::Fatal, $($arg)+);
        panic!();
    );
    ($($arg:tt)+) => (
        $crate::log!(gba::mgba::MGBADebugLevel::Fatal, $($arg)+);
        panic!();
    )
}
