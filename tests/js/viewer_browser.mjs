import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const TOLERANCE = 1.5;
const SKIP_CODE = 77;

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDir, "../..");

function skip(message) {
  console.error("skipping viewer browser regression: " + message);
  process.exit(SKIP_CODE);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function loadRoutes() {
  const files = {
    "/": { file: path.join(root, "tests/js/fixtures/viewer-browser.html"), type: "text/html; charset=utf-8" },
    "/tests/js/fixtures/viewer-browser.html": {
      file: path.join(root, "tests/js/fixtures/viewer-browser.html"),
      type: "text/html; charset=utf-8"
    },
    "/assets/site.css": { file: path.join(root, "assets/site.css"), type: "text/css; charset=utf-8" },
    "/assets/viewer.js": { file: path.join(root, "assets/viewer.js"), type: "text/javascript; charset=utf-8" }
  };
  const routes = new Map();
  for (const [routePath, entry] of Object.entries(files)) {
    routes.set(routePath, {
      type: entry.type,
      body: fs.readFileSync(entry.file, "utf8")
    });
  }
  return routes;
}

async function settle(page) {
  await page.evaluate(
    () =>
      new Promise((resolve) => {
        window.requestAnimationFrame(() => resolve());
      })
  );
}

async function readGeometry(page) {
  return page.evaluate(() => {
    const stage = document.querySelector(".viewer-stage");
    const image = document.querySelector(".viewer-image");
    if (!stage || !image) {
      return null;
    }
    const stageRect = stage.getBoundingClientRect();
    const imageRect = image.getBoundingClientRect();
    return {
      stage: {
        width: stageRect.width,
        height: stageRect.height,
        centerX: stageRect.left + stageRect.width / 2,
        centerY: stageRect.top + stageRect.height / 2
      },
      preview: {
        width: imageRect.width,
        height: imageRect.height,
        centerX: imageRect.left + imageRect.width / 2,
        centerY: imageRect.top + imageRect.height / 2
      },
      placeholder: image.classList.contains("is-placeholder"),
      objectFit: getComputedStyle(image).objectFit,
      immersive: document.querySelector(".viewer").classList.contains("is-immersive")
    };
  });
}

function expectedEdge(stage, item) {
  const fit = Math.min(stage.width / item.width, stage.height / item.height);
  return Math.min(item.width * fit, item.height * fit);
}

function assertPreview(label, geometry, item) {
  assert(geometry, label + ": the viewer did not render a preview image");
  assert(geometry.placeholder, label + ": the preview is not marked as a placeholder");

  const edge = expectedEdge(geometry.stage, item);
  const sizeMismatch = Math.max(
    Math.abs(geometry.preview.width - edge),
    Math.abs(geometry.preview.height - edge)
  );
  assert(
    sizeMismatch <= TOLERANCE,
    label +
      ": preview is " +
      geometry.preview.width.toFixed(2) +
      "x" +
      geometry.preview.height.toFixed(2) +
      " px but the fitted shorter edge is " +
      edge.toFixed(2) +
      " px (stage " +
      geometry.stage.width.toFixed(2) +
      "x" +
      geometry.stage.height.toFixed(2) +
      " px)"
  );

  const centerMismatch = Math.max(
    Math.abs(geometry.preview.centerX - geometry.stage.centerX),
    Math.abs(geometry.preview.centerY - geometry.stage.centerY)
  );
  assert(
    centerMismatch <= TOLERANCE,
    label +
      ": preview is off-center by " +
      centerMismatch.toFixed(2) +
      " px (preview center " +
      geometry.preview.centerX.toFixed(2) +
      "," +
      geometry.preview.centerY.toFixed(2) +
      ", stage center " +
      geometry.stage.centerX.toFixed(2) +
      "," +
      geometry.stage.centerY.toFixed(2) +
      ")"
  );

  assert(
    geometry.objectFit === "cover",
    label + ": placeholder object-fit is " + geometry.objectFit + " instead of cover"
  );
}

const landscape = { width: 4000, height: 3000 };
const portrait = { width: 3000, height: 4000 };

let playwright;
try {
  playwright = await import("playwright");
} catch (error) {
  skip("playwright is not installed (" + error.message + ")");
}

let browser;
try {
  browser = await playwright.chromium.launch({ headless: true });
} catch (error) {
  skip("chromium could not be launched (" + error.message + ")");
}

try {
  const routes = loadRoutes();
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const page = await context.newPage();

  await page.route("**/*", (route) => {
    const url = new URL(route.request().url());
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      return route.continue();
    }
    const entry = routes.get(url.pathname);
    if (entry) {
      return route.fulfill({
        status: 200,
        contentType: entry.type,
        body: entry.body
      });
    }
    // Originals and thumbnails outside the fixture stay unavailable, so the
    // placeholder is never replaced while geometry is asserted.
    return route.fulfill({
      status: 404,
      contentType: "text/plain; charset=utf-8",
      body: "not found"
    });
  });

  await page.goto("http://viewer.test/", { waitUntil: "load" });
  await page.locator("a.tile-link").first().waitFor();

  const assetsLoaded = await page.evaluate(() => {
    const stylesheet = Array.from(document.styleSheets).some(
      (sheet) => sheet.href && sheet.href.endsWith("/assets/site.css")
    );
    return {
      stylesheet,
      viewer: Boolean(window.__mediaCatalogViewer),
      tiles: document.querySelectorAll("a.tile-link").length
    };
  });
  assert(assetsLoaded.stylesheet, "the fixture did not load the real assets/site.css");
  assert(assetsLoaded.viewer, "the fixture did not load the real assets/viewer.js");
  assert(assetsLoaded.tiles === 2, "the fixture should provide landscape and portrait tiles");

  await page.locator("a.tile-link").first().click();
  await page.locator(".viewer[open]").waitFor({ state: "visible" });
  await page.locator(".viewer-image.is-placeholder").waitFor();
  await settle(page);
  assertPreview("initial landscape", await readGeometry(page), landscape);

  await page.keyboard.press("f");
  await settle(page);
  const immersive = await readGeometry(page);
  assert(immersive && immersive.immersive, "immersive mode did not activate");
  assertPreview("immersive landscape", immersive, landscape);

  await page.setViewportSize({ width: 900, height: 640 });
  await settle(page);
  assertPreview("resized immersive landscape", await readGeometry(page), landscape);

  await page.keyboard.press("ArrowRight");
  await settle(page);
  assertPreview("portrait", await readGeometry(page), portrait);

  await page.keyboard.press("f");
  await settle(page);
  const framed = await readGeometry(page);
  assert(framed && !framed.immersive, "immersive mode did not exit");
  assertPreview("framed portrait", framed, portrait);

  console.log("viewer browser regression passed");
  await context.close();
} catch (error) {
  console.error("viewer browser regression failed: " + (error && error.stack ? error.stack : error));
  process.exitCode = 1;
} finally {
  await browser.close();
}
