// Resolves download links against the latest GitHub release for OrbitSwitch
// and promotes the visitor's platform button. Falls back to GitHub Releases
// if the API is unreachable or has no matching installer asset.

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

function applyReleaseStatus(state) {
  window.__orbitswitchRelease = state;

  const lang = document.documentElement.lang === "zh-CN" ? "zh" : "en";
  for (const label of document.querySelectorAll("[data-version]")) {
    label.textContent = state.kind === "ready" ? state.tag : "GitHub Releases";
  }

  for (const note of document.querySelectorAll("[data-release-note]")) {
    if (state.kind === "ready") {
      note.textContent =
        lang === "zh"
          ? `已连接最新版本 ${state.tag} 安装包（macOS .dmg 与 Windows -setup.exe）。`
          : `Direct installers linked from latest release ${state.tag} (.dmg & -setup.exe).`;
    } else if (state.kind === "pending") {
      note.textContent =
        lang === "zh"
          ? "自动匹配 GitHub Releases 最新版本的 macOS .dmg 与 Windows -setup.exe 安装包。"
          : "Direct installers (.dmg & -setup.exe) resolve from the latest OrbitSwitch release on GitHub Releases.";
    } else {
      note.textContent =
        lang === "zh"
          ? "当前无法拉取最新构建信息，下载按钮已回退至 GitHub Releases 页面。"
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
