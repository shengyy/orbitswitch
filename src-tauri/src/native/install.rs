//! Discover the official CLI, its runtime `PATH`, and the desktop bundle without launching them.

use crate::provider::Provider;
use std::{
    cmp::Reverse,
    collections::HashSet,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cli {
    /// Entry path invoked for login commands (preserves symlink names and wrapper scripts).
    pub program: PathBuf,
    /// Canonical target executable, matched alongside `program` when scanning running processes.
    pub canonical: PathBuf,
    /// Complete `PATH` passed to CLI subprocesses so script runtimes (`node`, `bun`, etc.) resolve.
    pub path_env: OsString,
}

impl Cli {
    pub fn matches_exe(&self, exe: &Path) -> bool {
        self.program == exe || self.canonical == exe
    }
}

#[derive(Clone, Default)]
pub struct Install {
    pub cli: Option<Cli>,
    pub app: Option<PathBuf>,
    pub executable: Option<PathBuf>,
}

fn version_key(name: &str) -> (u32, u32, u32) {
    let trimmed = name
        .strip_prefix("node@")
        .or_else(|| name.strip_prefix('v'))
        .unwrap_or(name);
    let mut parts = trimmed.split('.').map(|part| {
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        digits.parse::<u32>().unwrap_or(0)
    });
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn versioned_bins(parent: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(prefix) {
                return None;
            }
            let dir = if suffix.is_empty() {
                entry.path()
            } else {
                entry.path().join(suffix)
            };
            dir.is_dir().then_some((name, dir))
        })
        .collect();
    found.sort_by_key(|(name, _)| (Reverse(version_key(name)), Reverse(name.clone())));
    found.into_iter().map(|(_, dir)| dir).collect()
}

fn dedup_absolute(candidates: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in candidates {
        if !path.as_os_str().is_empty() && path.is_absolute() && seen.insert(path.clone()) {
            out.push(path);
        }
    }
    out
}

fn env_dir(key: &str, sub: &str) -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os(key)?);
    base.is_absolute()
        .then(|| if sub.is_empty() { base } else { base.join(sub) })
}

#[cfg(not(unix))]
fn home_toolchain_dirs(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/bin"),
        home.join(".volta/bin"),
        home.join(".bun/bin"),
    ]
}

#[cfg(unix)]
fn home_toolchain_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        home.join(".local/bin"),
        home.join(".volta/bin"),
        home.join(".bun/bin"),
    ];
    dirs.push(home.join(".local/share/mise/shims"));
    dirs.extend(versioned_bins(
        &home.join(".local/share/mise/installs/node"),
        "",
        "bin",
    ));
    dirs.push(home.join(".asdf/shims"));
    dirs.extend(versioned_bins(
        &home.join(".asdf/installs/nodejs"),
        "",
        "bin",
    ));
    dirs.push(home.join(".nodenv/shims"));
    let mut fnm_roots = Vec::new();
    if let Some(root) = env_dir("FNM_DIR", "") {
        fnm_roots.push(root);
    }
    #[cfg(target_os = "macos")]
    {
        fnm_roots.push(home.join("Library/Application Support/fnm"));
    }
    fnm_roots.push(home.join(".local/share/fnm"));
    fnm_roots.push(home.join(".fnm"));
    for root in fnm_roots {
        dirs.push(root.join("aliases/default/bin"));
        dirs.extend(versioned_bins(
            &root.join("node-versions"),
            "v",
            "installation/bin",
        ));
    }
    let mut nvm_roots = Vec::new();
    if let Some(root) = env_dir("NVM_DIR", "") {
        nvm_roots.push(root);
    }
    nvm_roots.push(home.join(".nvm"));
    nvm_roots.push(home.join(".config/nvm"));
    for root in nvm_roots {
        dirs.extend(versioned_bins(&root.join("versions/node"), "v", "bin"));
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(home.join("Library/pnpm"));
    }
    dirs.extend([
        home.join(".local/share/pnpm"),
        home.join(".npm-global/bin"),
        home.join("n/bin"),
        home.join(".nix-profile/bin"),
    ]);
    dirs
}

#[cfg(target_os = "macos")]
fn macos_system_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut files = vec![PathBuf::from("/etc/paths")];
    if let Ok(entries) = std::fs::read_dir("/etc/paths.d") {
        let mut extra: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
        extra.sort();
        files.extend(extra);
    }
    for file in files {
        if let Ok(contents) = std::fs::read_to_string(file) {
            out.extend(
                contents
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from),
            );
        }
    }
    out
}

fn system_toolchain_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(unix)]
    {
        for prefix in [Path::new("/opt/homebrew"), Path::new("/usr/local")] {
            dirs.push(prefix.join("bin"));
            dirs.push(prefix.join("sbin"));
            dirs.push(prefix.join("opt/node/bin"));
            dirs.extend(versioned_bins(&prefix.join("opt"), "node@", "bin"));
        }
        dirs.extend([
            PathBuf::from("/nix/var/nix/profiles/default/bin"),
            PathBuf::from("/opt/local/bin"),
        ]);
        #[cfg(target_os = "macos")]
        {
            dirs.extend(macos_system_paths());
        }
        dirs.extend([
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
        ]);
    }
    #[cfg(windows)]
    {
        if let Some(app_data) = env_dir("APPDATA", "") {
            dirs.push(app_data.join("npm"));
            dirs.push(app_data.join("fnm/aliases/default"));
            dirs.extend(versioned_bins(
                &app_data.join("fnm/node-versions"),
                "v",
                "installation",
            ));
            dirs.extend(versioned_bins(&app_data.join("nvm"), "v", ""));
        }
        if let Some(local) = env_dir("LOCALAPPDATA", "") {
            dirs.push(local.join("pnpm"));
            dirs.push(local.join("Volta/bin"));
            dirs.push(local.join("fnm/aliases/default"));
            dirs.extend(versioned_bins(
                &local.join("fnm/node-versions"),
                "v",
                "installation",
            ));
        }
        if let Some(nvm_home) = env_dir("NVM_HOME", "") {
            dirs.extend(versioned_bins(&nvm_home, "v", ""));
        }
        if let Some(program_files) = env_dir("ProgramFiles", "nodejs") {
            dirs.push(program_files);
        }
    }
    dirs
}

fn collect_search_dirs(
    env_path: Option<&OsStr>,
    home: Option<&Path>,
    extra_env_dirs: impl IntoIterator<Item = PathBuf>,
    system_dirs: impl IntoIterator<Item = PathBuf>,
) -> Vec<PathBuf> {
    let mut candidates = env_path
        .map(|p| std::env::split_paths(p).collect::<Vec<_>>())
        .unwrap_or_default();
    candidates.extend(extra_env_dirs);
    if let Some(home) = home {
        candidates.extend(home_toolchain_dirs(home));
    }
    candidates.extend(system_dirs);
    dedup_absolute(candidates)
}

fn search_dirs() -> Vec<PathBuf> {
    let env_hints = [
        env_dir("NVM_BIN", ""),
        env_dir("NVM_SYMLINK", ""),
        env_dir("VOLTA_HOME", "bin"),
        env_dir("BUN_INSTALL", "bin"),
        env_dir("PNPM_HOME", ""),
    ]
    .into_iter()
    .flatten();
    collect_search_dirs(
        std::env::var_os("PATH").as_deref(),
        std::env::home_dir().as_deref(),
        env_hints,
        system_toolchain_dirs(),
    )
}

fn build_cli(program: PathBuf, dirs: &[PathBuf]) -> Option<Cli> {
    let mut path_dirs = dirs.to_vec();
    if let Some(parent) = program.parent().filter(|p| p.is_absolute())
        && !path_dirs.iter().any(|existing| existing == parent)
    {
        path_dirs.push(parent.to_path_buf());
    }
    let joinable: Vec<PathBuf> = path_dirs
        .into_iter()
        .filter(|dir| std::env::join_paths(std::slice::from_ref(dir)).is_ok())
        .collect();
    let path_env = std::env::join_paths(&joinable).ok()?;
    let canonical = program.canonicalize().unwrap_or_else(|_| program.clone());
    Some(Cli {
        program,
        canonical,
        path_env,
    })
}

fn locate_cli_in(name: &str, dirs: &[PathBuf]) -> Option<Cli> {
    let binary = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let program = dirs
        .iter()
        .map(|dir| dir.join(&binary))
        .find(|candidate| candidate.is_file())?;
    build_cli(program, dirs)
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
    let dirs = search_dirs();
    let mut install = Install {
        cli: locate_cli_in(name, &dirs),
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
                            install.cli = build_cli(p, &dirs);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_discovery_exports_fallback_runtime_dirs_and_drops_relative_entries() {
        let temp = tempfile::tempdir().unwrap();
        let gui_bin = temp.path().join("usr-bin");
        let brew_bin = temp.path().join("homebrew/bin");
        std::fs::create_dir_all(&gui_bin).unwrap();
        std::fs::create_dir_all(&brew_bin).unwrap();
        let binary_name = if cfg!(windows) { "codex.exe" } else { "codex" };
        let launcher = brew_bin.join(binary_name);
        std::fs::write(&launcher, "fixture").unwrap();

        let raw_path =
            std::env::join_paths([gui_bin.as_path(), Path::new("relative-dir")]).unwrap();
        let dirs = collect_search_dirs(Some(&raw_path), None, [], [brew_bin.clone()]);
        assert_eq!(dirs, vec![gui_bin.clone(), brew_bin.clone()]);

        let cli = locate_cli_in("codex", &dirs).expect("CLI should be found in fallback dir");
        assert_eq!(cli.program, launcher);
        assert!(cli.matches_exe(&cli.program));
        assert!(cli.matches_exe(&cli.canonical));
        let exported: Vec<PathBuf> = std::env::split_paths(&cli.path_env).collect();
        assert_eq!(exported, vec![gui_bin, brew_bin]);
    }

    #[test]
    fn versioned_node_bins_sort_by_numeric_semver_descending() {
        let temp = tempfile::tempdir().unwrap();
        let versions = temp.path().join("versions/node");
        for ver in ["v9.11.2", "v20.18.0", "v22.2.0", "v22.14.0"] {
            std::fs::create_dir_all(versions.join(ver).join("bin")).unwrap();
        }
        let ordered = versioned_bins(&versions, "v", "bin");
        assert_eq!(
            ordered,
            vec![
                versions.join("v22.14.0/bin"),
                versions.join("v22.2.0/bin"),
                versions.join("v20.18.0/bin"),
                versions.join("v9.11.2/bin"),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_cli_retains_entry_path_and_matches_canonical_target() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join("homebrew/bin");
        let pkg_dir = temp
            .path()
            .join("homebrew/lib/node_modules/@openai/codex/bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let target = pkg_dir.join("codex.js");
        std::fs::write(&target, "#!/usr/bin/env node\n").unwrap();
        let link = bin_dir.join("codex");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let cli = locate_cli_in("codex", std::slice::from_ref(&bin_dir)).unwrap();
        assert_eq!(cli.program, link);
        assert_eq!(cli.canonical, target.canonicalize().unwrap());
        assert!(cli.matches_exe(&link));
        assert!(cli.matches_exe(&cli.canonical));
    }

    #[cfg(target_os = "macos")]
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
