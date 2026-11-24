import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, statSync, writeFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const cacheSchema = 1;
const mode = process.argv[2];
const modes = new Set(["desktop:exe", "desktop:build", "android:build", "android:bundle"]);
if (!modes.has(mode)) {
  console.error(`Modo de build inválido: ${mode ?? "(vazio)"}`);
  process.exit(2);
}

const android = mode.startsWith("android:");
const extra = process.argv.slice(3);
const force = extra.includes("--force");
const forwarded = extra.filter((argument) => argument !== "--force");
const cacheable = forwarded.length === 0;
const tauriRoot = join(root, "src-tauri");
const version = JSON.parse(readFileSync(join(tauriRoot, "tauri.conf.json"), "utf8")).version;
const artifactRoot = join(tauriRoot, "target");
const outputs = {
  "desktop:exe": [join(artifactRoot, "release", "yt-dlp-deck.exe")],
  "desktop:build": [
    join(artifactRoot, "release", "yt-dlp-deck.exe"),
    join(artifactRoot, "release", "bundle", "nsis", `YT-DLP Deck_${version}_x64-setup.exe`),
  ],
  "android:build": [join(artifactRoot, "android", `YT-DLP-Deck-Android-v${version}-arm64.apk`)],
  "android:bundle": [join(artifactRoot, "android", `YT-DLP-Deck-Android-v${version}-arm64.aab`)],
}[mode];
function cachePath(forMode) {
  return join(root, "node_modules", ".cache", "yt-dlp-deck", `artifact-${forMode.replace(":", "-")}.json`);
}
const cacheFile = cachePath(mode);

const ignoredDirectories = new Set(["target", "gen", "build", ".gradle", ".kotlin", ".cxx", ".tauri", "node_modules"]);
const wasm = join(root, "public", "visual-core.wasm");
function collect(directory) {
  if (!existsSync(directory)) return [];
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      if (ignoredDirectories.has(entry.name)) return [];
      if (!android && (
        path === join(tauriRoot, "android") ||
        path === join(tauriRoot, "icons", "android") ||
        path === join(tauriRoot, "plugins", "mobile-downloader", "android")
      )) return [];
      return collect(path);
    }
    return entry.isFile() && path !== wasm ? [path] : [];
  });
}

const generatedAndroid = join(tauriRoot, "gen", "android");
const fixedFiles = [
  "index.html", "package.json", "package-lock.json", "tsconfig.json", "vite.config.ts",
  "scripts/build-frontend.mjs", ".env", ".env.local",
  ".env.production", ".env.production.local", "node_modules/.package-lock.json",
  "node_modules/@tauri-apps/cli/tauri.js",
].map((name) => join(root, name));
if (android) {
  fixedFiles.push(join(root, "scripts", "build-android.ps1"));
  for (const name of [
    "build.gradle.kts", "gradle.properties", "settings.gradle", "tauri.settings.gradle",
    "gradle/wrapper/gradle-wrapper.jar", "gradle/wrapper/gradle-wrapper.properties",
    "gradle/verification-metadata.xml", "app/build.gradle.kts",
    "app/tauri.build.gradle.kts", "app/src/main/AndroidManifest.xml",
  ]) fixedFiles.push(join(generatedAndroid, name));
  const sdk = process.env.ANDROID_HOME || join(process.env.LOCALAPPDATA || "", "Android", "Sdk");
  fixedFiles.push(join(sdk, "platforms", "android-36", "source.properties"));
  fixedFiles.push(join(sdk, "ndk", "27.3.13750724", "source.properties"));
  const signing = join(process.env.LOCALAPPDATA || "", "YT-DLP-Deck", "signing");
  fixedFiles.push(process.env.YTDLP_ANDROID_KEYSTORE || join(signing, "release.keystore"));
  fixedFiles.push(join(signing, "signing.properties"));
}

function sourceHash(forMode = mode) {
  const hash = createHash("sha256");
  hash.update(`artifact-cache-${cacheSchema}:${forMode}`);
  const paths = [
    ...collect(join(root, "src")), ...collect(join(root, "visual-core")),
    ...collect(join(root, "public")), ...collect(join(root, "dist")),
    ...collect(tauriRoot), ...fixedFiles.filter(existsSync),
  ];
  for (const path of [...new Set(paths)].sort()) {
    hash.update(relative(root, path));
    hash.update(readFileSync(path));
  }
  for (const [key, value] of Object.entries(process.env).sort(([a], [b]) => a.localeCompare(b))) {
    if (key === "NODE_ENV" || key === "RUSTFLAGS" || key === "RUSTUP_TOOLCHAIN" ||
        key === "CARGO_ENCODED_RUSTFLAGS" || key === "JAVA_HOME" ||
        key === "ANDROID_HOME" || key === "NDK_HOME" || key.startsWith("VITE_") ||
        key.startsWith("CARGO_PROFILE_RELEASE_") || key.startsWith("YTDLP_ANDROID_")) {
      hash.update(`${key}=${value}`);
    }
  }
  return hash.digest("hex");
}

function fileHash(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function outputState(paths = outputs) {
  if (!existsSync(wasm) || !paths.every(existsSync)) return null;
  return paths.map((path) => ({
    path: relative(root, path),
    size: statSync(path).size,
    sha256: fileHash(path),
  }));
}

if (!force && cacheable && existsSync(cacheFile)) {
  try {
    const cached = JSON.parse(readFileSync(cacheFile, "utf8"));
    if (cached.sourceHash === sourceHash() &&
        JSON.stringify(cached.outputs) === JSON.stringify(outputState())) {
      console.log("Build sem alterações; artefato validado:");
      for (const path of outputs) console.log(path);
      process.exit(0);
    }
  } catch {
    // Missing or damaged cache: run the normal build.
  }
}

const command = android ? "powershell.exe" : process.execPath;
const args = android
  ? ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", join(root, "scripts", "build-android.ps1"),
      ...(mode === "android:bundle" ? ["-Bundle"] : []), ...forwarded]
  : [join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"), "build",
      ...(mode === "desktop:exe" ? ["--no-bundle"] : []), ...forwarded];
const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);

if (cacheable) {
  const built = outputState();
  if (!built) {
    console.error("O build terminou sem gerar todos os artefatos esperados.");
    process.exit(1);
  }
  function saveCache(forMode, state) {
    const path = cachePath(forMode);
    mkdirSync(resolve(path, ".."), { recursive: true });
    const pending = `${path}.tmp`;
    writeFileSync(pending, JSON.stringify({ sourceHash: sourceHash(forMode), outputs: state }));
    renameSync(pending, path);
  }
  saveCache(mode, built);
  if (mode === "desktop:build") saveCache("desktop:exe", outputState([outputs[0]]));
}
