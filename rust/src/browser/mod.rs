//! Cookie types, local caches, and web probes.

#[cfg(windows)]
pub(crate) mod claude_desktop;
pub mod cookies;
pub mod detection;
pub mod session_scan;
pub mod watchdog;

// Re-exports for future UI integration
#[allow(
    unused_imports,
    reason = "watchdog API re-exported for future UI integration; no caller outside this module exists yet"
)]
pub use watchdog::{WatchdogConfig, WatchdogError, WebProbeWatchdog, global_watchdog};
