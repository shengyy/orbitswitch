//! Discover the official CLI and desktop bundle without launching them.
use crate::provider::Provider;
use std::path::{Path, PathBuf};
#[derive(Clone, Default)]
pub struct Install {
    pub cli: Option<PathBuf>,
    pub app: Option<PathBuf>,
    pub executable: Option<PathBuf>,
}
fn cli(name: &str) -> Option<PathBuf> {
    let mut dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(home) = std::env::home_dir() {
        dirs.extend([
            home.join(".local/bin"),
            home.join(".bun/bin"),
            home.join(".npm-global/bin"),
        ]);
    }
    dirs.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    dirs.into_iter()
        .map(|p| {
            p.join(if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.into()
            })
        })
        .find(|p| p.is_file())
        .map(|p| p.canonicalize().unwrap_or(p))
}
#[cfg(target_os = "macos")]
fn mac_bundle(provider: Provider, root: &Path) -> Option<PathBuf> {
    let value = |key: &str| {
        let output = std::process::Command::new("/usr/bin/plutil")
            .args(["-extract", key, "raw", "-o", "-"])
            .arg(root.join("Contents/Info.plist"))
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let expected = match provider {
        Provider::Codex => "com.openai.codex",
        Provider::Claude => "com.anthropic.claudefordesktop",
        _ => return None,
    };
    if value("CFBundleIdentifier")?.as_str() != expected {
        return None;
    }
    let name = value("CFBundleExecutable")?;
    if Path::new(&name).components().count() != 1 || name == "." || name == ".." {
        return None;
    }
    let exe = root.join("Contents/MacOS").join(name);
    exe.is_file().then_some(exe)
}
pub fn locate(provider: Provider) -> Install {
    let name = if provider == Provider::Codex {
        "codex"
    } else {
        "claude"
    };
    let mut install = Install {
        cli: cli(name),
        ..Install::default()
    };
    #[cfg(target_os = "macos")]
    {
        let names: &[&str] = if provider == Provider::Codex {
            &["Codex", "ChatGPT"]
        } else {
            &["Claude"]
        };
        for name in names {
            for dir in [
                Some(PathBuf::from("/Applications")),
                std::env::home_dir().map(|p| p.join("Applications")),
            ]
            .into_iter()
            .flatten()
            {
                let root = dir.join(format!("{name}.app"));
                if let Some(exe) = mac_bundle(provider, &root) {
                    install.app = Some(root.clone());
                    install.executable = Some(exe);
                    if install.cli.is_none() && provider == Provider::Codex {
                        let p = root.join("Contents/Resources/codex-cli/bin/codex");
                        if p.is_file() {
                            install.cli = Some(p);
                        }
                    }
                    return install;
                }
            }
        }
    }
    #[cfg(windows)]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(local);
        let title = if provider == Provider::Codex {
            "Codex"
        } else {
            "Claude"
        };
        let mut candidates = vec![
            root.join("Programs").join(title),
            root.join(title),
            if provider == Provider::Codex {
                root.join("OpenAI/Codex")
            } else {
                root.join("AnthropicClaude")
            },
        ];
        if let Ok(entries) = std::fs::read_dir(root.join(title)) {
            candidates.extend(
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().starts_with("app-"))
                    .map(|e| e.path()),
            );
        }
        for dir in candidates {
            let exe = dir.join(format!("{title}.exe"));
            if exe.is_file() {
                install.app = Some(dir);
                install.executable = Some(exe);
                break;
            }
        }
    }
    install
}
pub fn contains(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn codex_requires_bundle_identity_not_the_chatgpt_filename() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ChatGPT.app");
        std::fs::create_dir_all(root.join("Contents/MacOS")).unwrap();
        std::fs::write(root.join("Contents/MacOS/ChatGPT"), "fixture").unwrap();
        let plist = |id: &str| {
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{id}</string><key>CFBundleExecutable</key><string>ChatGPT</string></dict></plist>"
            )
        };
        std::fs::write(root.join("Contents/Info.plist"), plist("com.openai.chat")).unwrap();
        assert!(mac_bundle(Provider::Codex, &root).is_none());
        std::fs::write(root.join("Contents/Info.plist"), plist("com.openai.codex")).unwrap();
        assert_eq!(
            mac_bundle(Provider::Codex, &root),
            Some(root.join("Contents/MacOS/ChatGPT"))
        );
    }
}
