//! Loopback tests exercise the official updater, including its signature verifier.

use std::time::Duration;

use serde_json::json;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::UpdaterExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const PAYLOAD: &[u8] = b"Synthetic AgyOrbit updater test payload\n";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEREOEFBOThFNTc0OUI3QTgKUldTb3QwbFhqcW1LM1JnS252eER0ZkNPZEs0TWVSR0EvV0RMdFRTTUgrUUk0NUlIWUE4MkUxSjkK";
const SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTb3QwbFhqcW1LM2RyNUpJbmhzcFVGcjVVaXVTWjhCMFoxWUNyakp4NGtzM0J0Z1R4Q1ZMdTh3RWIwdFVPVnl5RzMycGlCQ2ovRU1nN0hKMTJSaUVmY2JQbHREZUszbXdjPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxMDk2NjcyCWZpbGU6cGF5bG9hZAl2ZXJzaW9uOjk5LjAuMAo1Q2FGWjdRa052SHFhUEx5Z1U5enJWQ0VkWlQzTExMNnkzdllSS00wOHVib2E4WEdWQmFaSnUyTnUwUjFNYnh0YVRVbUpwQUJsTXBlb1RWdFduMFZEZz09Cg==";

struct Server {
    endpoint: url::Url,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn with_artifact(version: &str, payload: &[u8], status: &str, signature: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint: url::Url = format!("http://{}/latest.json", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let artifact = json!({ "url": endpoint.join("package").unwrap(), "signature": signature });
        let manifest = json!({
            "version": version,
            "notes": "Synthetic release notes",
            "platforms": {
                "darwin-aarch64": artifact, "darwin-x86_64": artifact,
                "windows-x86_64": artifact,
            },
        })
        .to_string();
        let payload = payload.to_vec();
        let status = status.to_owned();
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = [0; 4096];
                let read = stream.read(&mut request).await.unwrap();
                let is_manifest =
                    String::from_utf8_lossy(&request[..read]).starts_with("GET /latest.json ");
                let body = if is_manifest {
                    manifest.as_bytes()
                } else {
                    &payload
                };
                let header = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(header.as_bytes()).await.unwrap();
                stream.write_all(body).await.unwrap();
            }
        });
        Self { endpoint, task }
    }

    async fn new(version: &str, payload: &[u8], status: &str) -> Self {
        Self::with_artifact(version, payload, status, SIGNATURE).await
    }

    async fn check(&self) -> tauri_plugin_updater::Result<Option<tauri_plugin_updater::Update>> {
        self.check_at(PUBLIC_KEY, None, None).await
    }

    async fn check_at(
        &self,
        pubkey: &str,
        target: Option<&str>,
        executable: Option<&std::path::Path>,
    ) -> tauri_plugin_updater::Result<Option<tauri_plugin_updater::Update>> {
        let mut context = mock_context(noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            json!({ "pubkey": pubkey, "requireSignedVersion": true }),
        );
        context.package_info_mut().version = env!("CARGO_PKG_VERSION").parse().unwrap();
        let app = mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let mut builder = app.updater_builder();
        if let Some(target) = target {
            builder = builder.target(target);
        }
        if let Some(executable) = executable {
            builder = builder.executable_path(executable);
        }
        builder
            .endpoints(vec![self.endpoint.clone()])?
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()?
            .check()
            .await
    }
}

#[tokio::test]
async fn valid_signed_download_reports_notes_and_progress() {
    let server = Server::new("99.0.0", PAYLOAD, "200 OK").await;
    let update = server.check().await.unwrap().unwrap();
    assert_eq!(update.body.as_deref(), Some("Synthetic release notes"));
    let mut downloaded = 0;
    let bytes = update
        .download(|n, _| downloaded += n, || {})
        .await
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    assert_eq!(downloaded, PAYLOAD.len());
}

#[tokio::test]
async fn tampered_payload_and_mismatched_signed_version_are_rejected() {
    let server = Server::new("99.0.0", b"tampered", "200 OK").await;
    let update = server.check().await.unwrap().unwrap();
    assert!(update.download(|_, _| {}, || {}).await.is_err());

    let server = Server::new("99.0.1", PAYLOAD, "200 OK").await;
    let update = server.check().await.unwrap().unwrap();
    assert!(update.download(|_, _| {}, || {}).await.is_err());
}

#[tokio::test]
async fn current_and_older_releases_are_not_offered_and_http_failure_is_an_error() {
    for version in [env!("CARGO_PKG_VERSION"), "0.0.1"] {
        let server = Server::new(version, PAYLOAD, "200 OK").await;
        assert!(server.check().await.unwrap().is_none());
    }
    let server = Server::new("99.0.0", PAYLOAD, "404 Not Found").await;
    assert!(server.check().await.is_err());
}

#[test]
fn production_configuration_requires_https_and_version_bound_signatures() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let updater: tauri_plugin_updater::Config =
        serde_json::from_value(config["plugins"]["updater"].clone()).unwrap();
    assert!(updater.require_signed_version);
    assert!(!updater.allow_downgrades);
    assert!(!updater.dangerous_accept_invalid_certs);
    assert!(!updater.dangerous_insecure_transport_protocol);
    assert!(updater.endpoints.iter().all(|url| url.scheme() == "https"));
}

#[cfg(target_os = "macos")]
#[test]
fn unbundled_executables_cannot_replace_their_build_directory() {
    assert!(!super::is_macos_bundle(std::path::Path::new(
        "target/debug/agyorbit"
    )));
    assert!(!super::is_macos_bundle(std::path::Path::new(
        "build/Contents/MacOS/agyorbit"
    )));
    assert!(super::is_macos_bundle(std::path::Path::new(
        "AgyOrbit.app/Contents/MacOS/agyorbit"
    )));
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn macos_installer_replaces_only_an_isolated_test_bundle() {
    use base64::Engine;
    const ARCHIVE_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDVDM0JFRURDQUNFNTkzMzEKUldReGsrV3MzTzQ3WEovNks0L0cyUG9uSFkrZ1poOHZMOXFKRVBWOGhlV0MwaVFVTisxVUQ5MXgK";
    const ARCHIVE_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVReGsrV3MzTzQ3WEtSYklFQ1NDRCt2ZHVlS2t3Sk1pcml6a0haejBFS0hVNGF0aGlXanIzUERlbVkrZERrMFY0MURkbmQwOVFpRTAwalZtc3R6QkxiNlRjQkJjUzZFakFzPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxMDk3MDMzCWZpbGU6dXBkYXRlZC5hcHAudGFyLmd6CXZlcnNpb246OTkuMC4wClNENndxZDhIU1Jqb0VrdW9PeFlzdTY4NlFYMDdCTFgvclcxZ0VuZ1Fnc3dNK1dCV0NOOUthUVU2ajRHdkY0dW03dE1FeitHSDhqVGNBMG8zemZDUkNRPT0K";
    const ARCHIVE: &str = "H4sICMn4wWoC/3VwZGF0ZWQuYXBwLnRhcgDt0jEOwjAMheEcJScgATUwI2bUgROENipd0qhxh9y+KQsSKxJC4v+WZz0PXnweSjvfR9n5lMxlihKiZHP1XXszfijTtlMfstXJuWdW72ntwb3mrd83rjkqbdUXLFn8XM+r/5RLlEeQsdNL6r2EXtdHUAAAAAAAAAAAAAAAAACAX7cCPdUQAAAoAAA=";
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("AgyOrbit.app/Contents/MacOS/agyorbit");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(&executable, b"synthetic old app").unwrap();
    let account_file = root.path().join("accounts.json");
    std::fs::write(&account_file, b"synthetic account data").unwrap();

    let server = Server::new("99.0.0", PAYLOAD, "200 OK").await;
    let invalid = server
        .check_at(PUBLIC_KEY, None, Some(&executable))
        .await
        .unwrap()
        .unwrap();
    let bytes = invalid.download(|_, _| {}, || {}).await.unwrap();
    assert!(invalid.install(bytes).is_err());
    assert_eq!(std::fs::read(&executable).unwrap(), b"synthetic old app");

    let archive = base64::engine::general_purpose::STANDARD
        .decode(ARCHIVE)
        .unwrap();
    let server = Server::with_artifact("99.0.0", &archive, "200 OK", ARCHIVE_SIGNATURE).await;
    let update = server
        .check_at(ARCHIVE_KEY, None, Some(&executable))
        .await
        .unwrap()
        .unwrap();
    let bytes = update.download(|_, _| {}, || {}).await.unwrap();
    update.install(bytes).unwrap();
    assert_eq!(
        std::fs::read(&executable).unwrap(),
        b"synthetic updated app"
    );
    assert_eq!(
        std::fs::read(&account_file).unwrap(),
        b"synthetic account data"
    );
}

#[tokio::test]
#[ignore = "requires downloaded release artifacts in AGYORBIT_RELEASE_DIR"]
async fn release_artifacts_verify() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("AGYORBIT_RELEASE_DIR").expect("set AGYORBIT_RELEASE_DIR"),
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("latest.json")).unwrap()).unwrap();
    let version = manifest["version"].as_str().unwrap();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert!(!manifest["notes"].as_str().unwrap().trim().is_empty());
    let release: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("release.json")).unwrap()).unwrap();
    assert_eq!(release["tag_name"], format!("v{version}"));
    let assets = release["assets"].as_array().unwrap();
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let pubkey = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    let mac = &manifest["platforms"]["darwin-aarch64"];
    assert_eq!(mac, &manifest["platforms"]["darwin-x86_64"]);
    for target in ["darwin-aarch64", "darwin-x86_64", "windows-x86_64"] {
        let artifact = &manifest["platforms"][target];
        let url = artifact["url"].as_str().unwrap();
        assert!(
            url.starts_with("https://api.github.com/repos/shengyy/orbitswitch/releases/assets/")
        );
        let asset = assets.iter().find(|asset| asset["url"] == url).unwrap();
        let file = asset["name"].as_str().unwrap();
        assert!(if target.starts_with("darwin") {
            file.ends_with(".app.tar.gz")
        } else {
            file.ends_with("-setup.exe")
        });
        let signature = artifact["signature"].as_str().unwrap();
        assert_eq!(
            signature.trim(),
            std::fs::read_to_string(directory.join(format!("{file}.sig")))
                .unwrap()
                .trim()
        );
        let bytes = std::fs::read(directory.join(file)).unwrap();
        let server = Server::with_artifact(version, &bytes, "200 OK", signature).await;
        let update = verify_update(pubkey, target, &server, None).await;
        assert_eq!(update.download(|_, _| {}, || {}).await.unwrap(), bytes);
        println!("Verified signed {target} artifact for {version}");
    }
}

async fn verify_update(
    pubkey: &str,
    target: &str,
    server: &Server,
    executable: Option<&std::path::Path>,
) -> tauri_plugin_updater::Update {
    let mut context = mock_context(noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        json!({ "pubkey": pubkey, "requireSignedVersion": true }),
    );
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    // This read-only verifier accepts the current release too. Production never permits downgrades.
    let mut builder = app
        .updater_builder()
        .target(target)
        .endpoints(vec![server.endpoint.clone()])
        .unwrap()
        .no_proxy()
        .version_comparator(|_, _| true);
    if let Some(executable) = executable {
        builder = builder.executable_path(executable);
    }
    builder.build().unwrap().check().await.unwrap().unwrap()
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "requires a local signed bundle in ORBITSWITCH_UPDATE_PACKAGE"]
async fn local_signed_bundle_installs() {
    let package = std::path::PathBuf::from(
        std::env::var_os("ORBITSWITCH_UPDATE_PACKAGE").expect("set ORBITSWITCH_UPDATE_PACKAGE"),
    );
    let signature = std::fs::read_to_string(format!("{}.sig", package.display())).unwrap();
    let bytes = std::fs::read(package).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let pubkey = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
    let server =
        Server::with_artifact(env!("CARGO_PKG_VERSION"), &bytes, "200 OK", &signature).await;
    let root = tempfile::tempdir().unwrap();
    let bundle = root.path().join("AgyOrbit.app");
    let executable = bundle.join("Contents/MacOS/agyorbit");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(&executable, b"synthetic old app").unwrap();
    let account_file = root.path().join("accounts.json");
    std::fs::write(&account_file, b"synthetic account data").unwrap();
    let update = verify_update(pubkey, "darwin-aarch64", &server, Some(&executable)).await;
    let verified = update.download(|_, _| {}, || {}).await.unwrap();
    assert_eq!(verified, bytes);
    update.install(verified).unwrap();
    assert!(
        std::process::Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&bundle)
            .status()
            .unwrap()
            .success()
    );
    let info = std::process::Command::new("plutil")
        .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .unwrap();
    assert!(info.status.success());
    assert_eq!(
        String::from_utf8(info.stdout).unwrap().trim(),
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(
        std::fs::read(&account_file).unwrap(),
        b"synthetic account data"
    );
}
