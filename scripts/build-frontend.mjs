import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const cacheFile = join(root, "node_modules", ".cache", "yt-dlp-deck", "frontend.json");
const dist = join(root, "dist");
const wasm = join(root, "public", "visual-core.wasm");
const force = process.argv.includes("--force");

function collect(directory) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return collect(path);
    return entry.isFile() && path !== wasm ? [path] : [];
  });
}

const sources = [
  ...collect(join(root, "src")),
  ...collect(join(root, "visual-core")),
  ...collect(join(root, "public")),
  ...[
    "index.html", "package.json", "package-lock.json", "tsconfig.json",
    "vite.config.ts", "scripts/build-frontend.mjs", ".env", ".env.local",
    ".env.production", ".env.production.local", "node_modules/.package-lock.json",
  ].map((name) => join(root, name)).filter(existsSync),
].sort();

const fingerprint = createHash("sha256");
for (const path of sources) {
  fingerprint.update(relative(root, path));
  fingerprint.update(readFileSync(path));
}
for (const [key, value] of Object.entries(process.env).sort(([a], [b]) => a.localeCompare(b))) {
  if (key === "NODE_ENV" || key.startsWith("VITE_")) {
    fingerprint.update(`${key}=${value}`);
  }
}
const inputHash = fingerprint.digest("hex");

function outputs() {
  if (!existsSync(wasm) || !existsSync(join(dist, "index.html"))) return null;
  return [wasm, ...collect(dist)].sort().map((path) => ({
    path: relative(root, path),
    size: statSync(path).size,
  }));
}

if (!force && existsSync(cacheFile)) {
  try {
    const cached = JSON.parse(readFileSync(cacheFile, "utf8"));
    if (cached.inputHash === inputHash &&
        JSON.stringify(cached.outputs) === JSON.stringify(outputs())) {
      console.log("Frontend sem alterações; usando o build existente.");
      process.exit(0);
    }
  } catch {
    // A cache miss simply rebuilds the frontend.
  }
}

const windows = process.platform === "win32";
const result = spawnSync(windows ? "cmd.exe" : "npm",
  windows ? ["/d", "/s", "/c", "npm run build:frontend"] : ["run", "build:frontend"], {
  cwd: root,
  stdio: "inherit",
});
if (result.status !== 0) process.exit(result.status ?? 1);

mkdirSync(resolve(cacheFile, ".."), { recursive: true });
writeFileSync(cacheFile, JSON.stringify({ inputHash, outputs: outputs() }));
