/*
 Custom debug! macro to avoid compiling with max_level_debug in debug builds for the log crate.

 max_level_debug results in many additional no-op calls from dependency libraries
 that makes the debug build run much slower.
*/

#[macro_export]
macro_rules! i_debug {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(debug_assertions)]
        log::info!(target: $target, $($arg)+)
    );
    ($($arg:tt)+) => (
        #[cfg(debug_assertions)]
        log::info!($($arg)+)
    )
}
