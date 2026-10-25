//! Open at Login.
//!
//! macOS registers the app itself through SMAppService, so System Settings
//! lists OrbitSwitch by name and icon. Its popover starts hidden anyway, so no
//! launch arguments are needed. Windows uses the per-user Run key and passes
//! `--background` so the window starts hidden.

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

use crate::error::{Error, Result};

fn launcher() -> Result<AutoLaunch> {
    let mut builder = AutoLaunchBuilder::new();
    #[cfg(target_os = "macos")]
    builder.set_macos_launch_mode(auto_launch::MacOSLaunchMode::SMAppService);
    #[cfg(not(target_os = "macos"))]
    {
        let exe = std::env::current_exe()?;
        builder
            .set_app_name("OrbitSwitch")
            .set_app_path(&exe.to_string_lossy())
            .set_args(&[crate::shell::BACKGROUND_ARG]);
    }
    builder
        .build()
        .map_err(|err| Error::Invalid(err.to_string()))
}

pub fn is_enabled() -> Result<bool> {
    launcher()?
        .is_enabled()
        .map_err(|err| Error::Invalid(err.to_string()))
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let launcher = launcher()?;
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|err| Error::Invalid(err.to_string()))
}
