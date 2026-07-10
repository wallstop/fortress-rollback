import { test, expect } from "@playwright/test";
import { createReadStream, promises as fs } from "node:fs";
import http from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RESULT_KEY = "__FORTRESS_ROLLBACK_RESULT__";
const GODOT_VERSION = "4.6.3-stable (official)";
const FIXTURE_ROOT = path.dirname(fileURLToPath(import.meta.url));
const DIST_ROOT = path.join(FIXTURE_ROOT, "dist");
const MIME_TYPES = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".pck", "application/octet-stream"],
  [".png", "image/png"],
  [".wasm", "application/wasm"],
]);
const MODES = [
  { name: "threaded", rustMode: "threaded", godotThreads: true },
  { name: "nothreads", rustMode: "nothreads", godotThreads: false },
];

function describeError(error) {
  return JSON.stringify({
    name: error?.name ?? "unknown",
    message: error?.message ?? String(error),
    stack: error?.stack ?? "",
  });
}

let server;
let baseUrl;

function isolationHeaders(contentType) {
  return {
    "Cache-Control": "no-store",
    "Content-Type": contentType,
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Resource-Policy": "same-origin",
  };
}

async function serve(request, response) {
  const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
  let pathname;
  try {
    pathname = decodeURIComponent(requestUrl.pathname);
  } catch {
    response.writeHead(400, isolationHeaders("text/plain; charset=utf-8"));
    response.end("invalid URL encoding");
    return;
  }

  if (pathname.endsWith("/")) {
    pathname += "index.html";
  }
  const filePath = path.resolve(DIST_ROOT, `.${pathname}`);
  if (!filePath.startsWith(`${DIST_ROOT}${path.sep}`)) {
    response.writeHead(403, isolationHeaders("text/plain; charset=utf-8"));
    response.end("forbidden");
    return;
  }

  try {
    const stat = await fs.stat(filePath);
    if (!stat.isFile()) {
      throw new Error("not a file");
    }
    const contentType = MIME_TYPES.get(path.extname(filePath)) ?? "application/octet-stream";
    response.writeHead(200, {
      ...isolationHeaders(contentType),
      "Content-Length": stat.size,
    });
    createReadStream(filePath).pipe(response);
  } catch {
    response.writeHead(404, isolationHeaders("text/plain; charset=utf-8"));
    response.end("not found");
  }
}

test.beforeAll(async () => {
  server = http.createServer((request, response) => {
    void serve(request, response);
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (typeof address === "string" || address === null) {
    throw new Error("HTTP server did not expose a TCP address");
  }
  baseUrl = `http://127.0.0.1:${address.port}`;
});

test.afterAll(async () => {
  if (server !== undefined) {
    await new Promise((resolve, reject) => {
      server.close((error) => (error ? reject(error) : resolve()));
    });
  }
});

for (const mode of MODES) {
  test(`${mode.name} Godot export completes Fortress quality probes`, async ({ page }, testInfo) => {
    test.setTimeout(60_000);
    const events = [];
    const errors = [];
    page.on("console", (message) => {
      const event = `[console:${message.type()}] ${message.text()}`;
      events.push(event);
      if (message.type() === "error") {
        errors.push(event);
      }
    });
    page.on("pageerror", (error) => {
      const event = `[pageerror] ${describeError(error)}`;
      events.push(event);
      errors.push(event);
    });
    page.on("requestfailed", (request) => {
      const event = `[requestfailed] ${request.url()} ${request.failure()?.errorText ?? "unknown"}`;
      events.push(event);
      errors.push(event);
    });
    page.on("response", (response) => {
      if (response.status() >= 400) {
        const event = `[response:${response.status()}] ${response.url()}`;
        events.push(event);
        errors.push(event);
      }
    });

    await page.addInitScript((resultKey) => {
      globalThis[resultKey] = { status: "pending" };
    }, RESULT_KEY);

    try {
      await page.goto(`${baseUrl}/${mode.name}/index.html`, { waitUntil: "load" });
      await page.waitForFunction(
        (resultKey) => globalThis[resultKey]?.status === "complete",
        RESULT_KEY,
        { timeout: 30_000 },
      );

      const result = await page.evaluate((resultKey) => globalThis[resultKey], RESULT_KEY);
      const crossOriginIsolated = await page.evaluate(() => globalThis.crossOriginIsolated);
      await testInfo.attach("browser-events", {
        body: Buffer.from(events.join("\n"), "utf8"),
        contentType: "text/plain",
      });

      expect(crossOriginIsolated).toBe(true);
      expect(result).toMatchObject({
        status: "complete",
        ok: true,
        mode: mode.rustMode,
        target_os: "emscripten",
        godot_version: GODOT_VERSION,
        godot_threads: mode.godotThreads,
        real_clock_smoke: true,
        ping_a_ms: 50,
        ping_b_ms: 50,
        error: "",
      });
      expect(result.real_clock_send_delta).toBeGreaterThanOrEqual(2);
      expect(errors).toEqual([]);
    } catch (error) {
      const snapshot = await page.evaluate((resultKey) => ({
        result: globalThis[resultKey],
        statusNotice: document.querySelector("#status-notice")?.textContent ?? "",
        statusVisibility: document.querySelector("#status")?.style.visibility ?? "removed",
      }), RESULT_KEY).catch((snapshotError) => ({
        snapshotError: describeError(snapshotError),
      }));
      events.push(`[snapshot] ${JSON.stringify(snapshot)}`);
      await testInfo.attach("browser-events", {
        body: Buffer.from(events.join("\n"), "utf8"),
        contentType: "text/plain",
      });
      throw error;
    }
  });
}
