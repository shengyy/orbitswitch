// Resolves download links against the latest GitHub release for OrbitSwitch
// and promotes the visitor's platform button. If the API is unreachable,
// rate-limited, or has no matching installer asset, links fall back cleanly to
// the GitHub Releases page and surface the exact release status.

const REPO = "shengyy/orbitswitch";
const RELEASES_URL = `https://github.com/${REPO}/releases`;

function preferWindows() {
  const platform = navigator.userAgentData?.platform ?? navigator.platform ?? "";
  return /win/i.test(platform);
}

function setLinks(kind, url) {
  const target = url || RELEASES_URL;
  for (const link of document.querySelectorAll(`[data-download="${kind}"]`)) {
    link.href = target;
  }
}

function isLegacyRelease(release) {
  const tag = String(release?.tag_name ?? "");
  const name = String(release?.name ?? "");
  const assets = Array.isArray(release?.assets) ? release.assets : [];
  if (/^v?0\.1\./i.test(tag)) return true;
  if (/agyorbit/i.test(name) && !/orbitswitch/i.test(name)) return true;
  return assets.length > 0 && assets.every((a) => /agyorbit/i.test(String(a.name ?? "")));
}

function applyReleaseStatus(state) {
  window.__orbitswitchRelease = state;
  document.documentElement.dataset.releaseState = state.kind;

  const lang = document.documentElement.lang === "zh-CN" ? "zh" : "en";
  for (const label of document.querySelectorAll("[data-version]")) {
    if (state.kind === "ready") {
      label.textContent = state.tag;
    } else if (state.kind === "legacy") {
      label.textContent = lang === "zh" ? `${state.tag}（旧版 AgyOrbit）` : `${state.tag} (legacy AgyOrbit)`;
    } else {
      label.textContent = lang === "zh" ? "GitHub Releases" : "GitHub Releases";
    }
  }

  for (const note of document.querySelectorAll("[data-release-note]")) {
    if (state.kind === "ready") {
      note.textContent =
        lang === "zh"
          ? `已连接最新版本 ${state.tag} 安装包（macOS .dmg 与 Windows -setup.exe）。`
          : `Direct installers linked from latest release ${state.tag} (.dmg & -setup.exe).`;
    } else if (state.kind === "legacy") {
      note.textContent =
        lang === "zh"
          ? `当前已发布标签 ${state.tag} 仍为单平台旧版 AgyOrbit（不含三 Tab 新能力）；OrbitSwitch 新版发布后将在此自动切换，也可前往 Releases 查看。`
          : `Published tag ${state.tag} is the legacy Antigravity-only build (without 3-tab support); new OrbitSwitch installers link automatically once released on GitHub Releases.`;
    } else if (state.kind === "pending") {
      note.textContent =
        lang === "zh"
          ? "自动匹配 GitHub Releases 最新版本的 macOS .dmg 与 Windows -setup.exe 安装包。"
          : "Direct installers (.dmg & -setup.exe) resolve from the latest OrbitSwitch release on GitHub Releases.";
    } else {
      note.textContent =
        lang === "zh"
          ? "当前无法直接拉取最新构建信息，下载按钮已回退至 GitHub Releases 页面。"
          : "Latest build metadata unavailable right now — download buttons open GitHub Releases directly.";
    }
  }
}

window.__refreshReleaseCopy = () => {
  if (window.__orbitswitchRelease) {
    applyReleaseStatus(window.__orbitswitchRelease);
  }
};

if (preferWindows()) {
  const mac = document.querySelector('.downloads [data-download="mac"]');
  const win = document.querySelector('.downloads [data-download="win"]');
  mac?.classList.remove("primary");
  win?.classList.add("primary");
  win?.parentElement?.prepend(win);
}

// Ensure safe initial fallback to Releases before network resolves.
setLinks("mac", RELEASES_URL);
setLinks("win", RELEASES_URL);
window.__orbitswitchRelease = { kind: "pending", tag: "GitHub Releases" };
applyReleaseStatus(window.__orbitswitchRelease);

fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
  headers: { Accept: "application/vnd.github+json" },
})
  .then((response) => (response.ok ? response.json() : Promise.reject(response.status)))
  .then((release) => {
    const assets = Array.isArray(release.assets) ? release.assets : [];
    const asset = (pattern) => assets.find((a) => pattern.test(a.name))?.browser_download_url;
    const macUrl = asset(/\.dmg$/i);
    const winUrl = asset(/setup\.exe$/i) ?? asset(/\.msi$/i);
    const legacy = isLegacyRelease(release);

    if (legacy) {
      // Do not silently serve a single-platform 0.1.x installer as if it were the 3-tab OrbitSwitch build;
      // route to the Releases page where build notes and versions are explicit.
      setLinks("mac", release.html_url || RELEASES_URL);
      setLinks("win", release.html_url || RELEASES_URL);
      applyReleaseStatus({ kind: "legacy", tag: release.tag_name || "v0.1.x" });
      return;
    }

    setLinks("mac", macUrl || release.html_url || RELEASES_URL);
    setLinks("win", winUrl || release.html_url || RELEASES_URL);
    applyReleaseStatus({
      kind: macUrl || winUrl ? "ready" : "fallback",
      tag: release.tag_name || "Latest release",
    });
  })
  .catch(() => {
    setLinks("mac", RELEASES_URL);
    setLinks("win", RELEASES_URL);
    applyReleaseStatus({ kind: "fallback", tag: "GitHub Releases" });
  });
