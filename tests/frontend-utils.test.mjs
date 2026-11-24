import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { runInNewContext } from "node:vm";
import ts from "typescript";

// Load only pure TypeScript modules. No WebView, Tauri or bundling required.
function loadUtils(path) {
  const source = readFileSync(new URL(path, import.meta.url), "utf8");
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  });
  const exports = {};
  runInNewContext(outputText, { exports, Date, Intl, Map });
  return exports;
}

const consoleUtils = loadUtils("../src/features/downloads/consoleUtils.ts");
const formats = loadUtils("../src/features/downloads/formatUtils.ts");
const history = loadUtils("../src/features/history/historyUtils.ts");

test("console distinguishes error, warning, progress and information", () => {
  const tone = consoleUtils.getConsoleLineTone;
  assert.equal(tone("ERROR: [download] 40% failed"), "error");
  assert.equal(tone("WARNING: retrying"), "warning");
  assert.equal(tone("[download] 40%"), "progress");
  assert.equal(tone("[sistema] Preparando"), "info");
  assert.equal(tone("ordinary line"), "default");
});

test("telemetry uses the newest download line", () => {
  const data = consoleUtils.readDownloadTelemetry([
    "[download] 10% of 12.00MiB at 1.00MiB/s ETA 00:10",
    "[download] 80% of ~12.00MiB at 2.50MiB/s ETA 00:02",
    "[info] metadata",
  ]);
  assert.equal(data.size, "12.00MiB");
  assert.equal(data.speed, "2.50MiB/s");
  assert.equal(data.eta, "00:02");
});

test("missing telemetry keeps safe display defaults", () => {
  const data = consoleUtils.readDownloadTelemetry([]);
  assert.equal(data.size, "Calculando");
  assert.equal(data.speed, "Calculando");
  assert.equal(data.eta, "—");
});

test("format classification and cover availability stay consistent", () => {
  for (const format of ["mp3", "flac", "wav", "m4a"]) assert.ok(formats.isAudio(format));
  assert.equal(formats.isAudio("mp4"), false);
  assert.equal(formats.isAudio(null), false);
  assert.ok(formats.isImage("image"));
  assert.ok(formats.supportsImageCover("Instagram"));
  assert.ok(formats.supportsImageCover("TikTok"));
  assert.equal(formats.supportsImageCover("YouTube"), false);
});

test("history groups by date without changing item order", () => {
  const now = new Date();
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1, 12).getTime();
  const items = [{ id: "a", createdAt: now.getTime() }, { id: "b", createdAt: now.getTime() }, { id: "c", createdAt: yesterday }];
  const groups = history.groupHistory(items);
  assert.equal(groups.length, 2);
  assert.equal(groups[0].label, "Hoje");
  assert.equal(groups[0].records[0].id, "a");
  assert.equal(groups[0].records[1].id, "b");
  assert.equal(groups[1].label, "Ontem");
  assert.equal(history.groupHistory([]).length, 0);
});
