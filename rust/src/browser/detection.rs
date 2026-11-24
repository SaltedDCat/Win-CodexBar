//! Browser labels and profile descriptors for injected cookie stores.

use std::path::PathBuf;

/// Supported browser types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserType {
    Chrome,
    ChromeBeta,
    ChromeDev,
    ChromeCanary,
    ChromeForTesting,
    Edge,
    Brave,
    Arc,
    Firefox,
    Chromium,
    /// Opera Stable. Its profile sits directly under the Roaming root, not in
    /// a `Default` subfolder, so the root is treated as the profile.
    Opera,
    /// Opera GX Stable, which uses the same layout as [`BrowserType::Opera`].
    OperaGx,
    /// Opera Neon Stable, which uses the same layout as [`BrowserType::Opera`].
    OperaNeon,
    /// Aside, a Chromium-based browser with the standard `User Data` layout.
    Aside,
    /// The Claude desktop app's Electron cookie store. Deliberately absent
    /// from [`BrowserType::all`]: only Claude reads it, and only when the
    /// user picks it, so generic browser scans never touch it.
    ClaudeDesktop,
}

impl BrowserType {
    /// Every web browser that generic cookie scans may read. App cookie
    /// stores such as [`BrowserType::ClaudeDesktop`] are excluded.
    pub fn all() -> &'static [BrowserType] {
        &[
            BrowserType::Chrome,
            BrowserType::ChromeBeta,
            BrowserType::ChromeDev,
            BrowserType::ChromeCanary,
            BrowserType::ChromeForTesting,
            BrowserType::Edge,
            BrowserType::Brave,
            BrowserType::Arc,
            BrowserType::Firefox,
            BrowserType::Chromium,
            BrowserType::Opera,
            BrowserType::OperaGx,
            BrowserType::OperaNeon,
            BrowserType::Aside,
        ]
    }

    /// Check if this is a Chromium-based browser
    pub fn is_chromium_based(&self) -> bool {
        !matches!(self, BrowserType::Firefox)
    }

    /// Get the display name
    pub fn display_name(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "Google Chrome",
            BrowserType::ChromeBeta => "Google Chrome Beta",
            BrowserType::ChromeDev => "Google Chrome Dev",
            BrowserType::ChromeCanary => "Google Chrome Canary",
            BrowserType::ChromeForTesting => "Chrome for Testing",
            BrowserType::Edge => "Microsoft Edge",
            BrowserType::Brave => "Brave",
            BrowserType::Arc => "Arc",
            BrowserType::Firefox => "Firefox",
            BrowserType::Chromium => "Chromium",
            BrowserType::Opera => "Opera",
            BrowserType::OperaGx => "Opera GX",
            BrowserType::OperaNeon => "Opera Neon",
            BrowserType::Aside => "Aside",
            BrowserType::ClaudeDesktop => "Claude desktop app",
        }
    }

    /// Stable browser identifier shared by the core and desktop IPC bridge.
    pub fn key(&self) -> &'static str {
        match self {
            BrowserType::Chrome => "chrome",
            BrowserType::ChromeBeta => "chrome-beta",
            BrowserType::ChromeDev => "chrome-dev",
            BrowserType::ChromeCanary => "chrome-canary",
            BrowserType::ChromeForTesting => "chrome-for-testing",
            BrowserType::Edge => "edge",
            BrowserType::Brave => "brave",
            BrowserType::Arc => "arc",
            BrowserType::Firefox => "firefox",
            BrowserType::Chromium => "chromium",
            BrowserType::Opera => "opera",
            BrowserType::OperaGx => "opera-gx",
            BrowserType::OperaNeon => "opera-neon",
            BrowserType::Aside => "aside",
            BrowserType::ClaudeDesktop => "claude-desktop",
        }
    }
}

/// A detected browser installation
#[derive(Debug, Clone)]
pub struct DetectedBrowser {
    pub browser_type: BrowserType,
    pub user_data_dir: PathBuf,
    pub profiles: Vec<BrowserProfile>,
}

/// A browser profile
#[derive(Debug, Clone)]
pub struct BrowserProfile {
    pub name: String,
    pub path: PathBuf,
    pub is_default: bool,
}
