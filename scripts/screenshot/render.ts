// Renders the README and site screenshots from the real UI with demo data.
//   bun scripts/screenshot/render.ts
// Needs Google Chrome and ImageMagick (`magick`). Renders at 3x and writes
// assets/screenshots/{hero,flow,codex,claude,adding}-{light,dark}@3x.png plus 2x
// copies without the suffix.

import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { $ } from "bun";

const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const here = import.meta.dir;
const repo = join(here, "../..");
const out = join(repo, "assets/screenshots");
const site = await mkdtemp(join(tmpdir(), "orbitswitch-site-"));
const profile = await mkdtemp(join(tmpdir(), "orbitswitch-chrome-"));

// 1. Build the real frontend and stage it under a menu bar with the mocked IPC.
await $`bunx vite build --base ./ --outDir ${site} --emptyOutDir`.cwd(repo).quiet();
const tray = (await Bun.file(join(repo, "assets/brand/tray-template.svg")).text())
  .replace(/<!--[\s\S]*?-->/g, "")
  .replaceAll("#000", "currentColor");
const index = (await Bun.file(join(site, "index.html")).text())
  .replace("</head>", '<link rel="stylesheet" href="frame.css"><script src="mock-tauri.js"></script></head>')
  .replace(
    '<div id="root"></div>',
    `<div class="stage"><div class="menubar"><span class="tray">${tray}</span><span class="clock"></span></div><div id="root"></div></div>`,
  );
await Bun.write(join(site, "index.html"), index);
await Bun.write(join(site, "mock-tauri.js"), Bun.file(join(here, "mock-tauri.js")));
await Bun.write(join(site, "frame.css"), Bun.file(join(here, "frame.css")));

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  async fetch(request) {
    const path = new URL(request.url).pathname;
    const file = Bun.file(join(site, path === "/" ? "index.html" : path));
    return (await file.exists()) ? new Response(file) : new Response("not found", { status: 404 });
  },
});

// 2. Drive Chrome over the DevTools protocol.
const chrome = Bun.spawn(
  [
    CHROME,
    "--headless=new",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-component-update",
    "--disable-background-networking",
    "--hide-scrollbars",
    "about:blank",
  ],
  { stderr: "pipe", stdout: "ignore" },
);
const port = await (async () => {
  const decoder = new TextDecoder();
  let log = "";
  for await (const chunk of chrome.stderr) {
    log += decoder.decode(chunk);
    const match = log.match(/DevTools listening on ws:\/\/127\.0\.0\.1:(\d+)\//);
    if (match) return match[1];
  }
  throw new Error("Chrome did not start");
})();
const targets = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()) as {
  type: string;
  webSocketDebuggerUrl: string;
}[];
const page = targets.find((t) => t.type === "page");
if (!page) throw new Error("no page target");

const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve) => socket.addEventListener("open", resolve, { once: true }));
let nextId = 1;
const pending = new Map<number, (result: Record<string, unknown>) => void>();
socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  pending.get(message.id)?.(message.result ?? {});
  pending.delete(message.id);
});
const send = (method: string, params: Record<string, unknown> = {}) =>
  new Promise<Record<string, unknown>>((resolve) => {
    const id = nextId++;
    pending.set(id, resolve);
    socket.send(JSON.stringify({ id, method, params }));
  });
const evaluate = async (expression: string) =>
  ((await send("Runtime.evaluate", { expression, returnByValue: true })).result as { value: unknown }).value;

async function shoot(scene: string, scheme: "light" | "dark", ready: string, file: string) {
  await send("Emulation.setDeviceMetricsOverride", {
    width: 720,
    height: 920,
    deviceScaleFactor: 3,
    mobile: false,
  });
  await send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });
  await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: scheme }] });
  await send("Page.navigate", { url: `http://127.0.0.1:${server.port}/?scene=${scene}` });
  const deadline = Date.now() + 10_000;
  while (!(await evaluate(`!!document.querySelector(${JSON.stringify(ready)})`))) {
    if (Date.now() > deadline) throw new Error(`${scene}: ${ready} never appeared`);
    await Bun.sleep(100);
  }
  await Bun.sleep(700); // let entrance animations finish
  const box = JSON.parse(
    String(await evaluate("JSON.stringify(document.querySelector('.stage').getBoundingClientRect())")),
  ) as { x: number; y: number; width: number; height: number };
  const margin = 72; // room for the whole stage shadow (0 18px 50px)
  const shot = await send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: true,
    clip: {
      x: box.x - margin,
      y: box.y - margin,
      width: box.width + margin * 2,
      height: box.height + margin * 2,
      scale: 1,
    },
  });
  await Bun.write(file, Buffer.from(String(shot.data), "base64"));
}

// 3. Account lists and sheets in both appearances, at 3x and 2x.
const work = await mkdtemp(join(tmpdir(), "orbitswitch-shots-"));
const publish = async (source: string, name: string) => {
  await $`cp ${source} ${join(out, `${name}@3x.png`)}`.quiet();
  await $`magick ${source} -resize 66.6667% ${join(out, `${name}.png`)}`.quiet();
};
try {
  for (const scheme of ["light", "dark"] as const) {
    await shoot("list", scheme, ".card .tag-best", join(work, "hero.png"));
    await publish(join(work, "hero.png"), `hero-${scheme}`);
    for (const provider of ["codex", "claude"]) {
      await shoot(`list&provider=${provider}`, scheme, ".card .tag-best", join(work, `${provider}.png`));
      await publish(join(work, `${provider}.png`), `${provider}-${scheme}`);
    }
    await shoot("confirm", scheme, ".sheet h2", join(work, "confirm.png"));
    await shoot("switching", scheme, ".steps .now", join(work, "switching.png"));
    await $`magick ${join(work, "confirm.png")} ${join(work, "switching.png")} -background none +append ${join(work, "flow.png")}`.quiet();
    await publish(join(work, "flow.png"), `flow-${scheme}`);
    await shoot("adding", scheme, ".sheet .btn-primary:not(:disabled)", join(work, "adding.png"));
    await publish(join(work, "adding.png"), `adding-${scheme}`);
  }
} finally {
  socket.close();
  chrome.kill();
  server.stop(true);
  await Promise.all([site, profile, work].map((dir) => rm(dir, { recursive: true, force: true })));
}
console.log("wrote {hero,flow,codex,claude,adding}-{light,dark}{,@3x}.png to assets/screenshots");
