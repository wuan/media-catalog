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

async function readControls(page) {
  return page.evaluate(() => {
    function describe(element) {
      if (!element) {
        return null;
      }
      const icon = element.querySelector("svg.viewer-icon");
      const children = icon
        ? Array.from(icon.children).map((child) => ({
            tag: child.tagName.toLowerCase(),
            d: child.getAttribute("d")
          }))
        : [];
      const rect = element.getBoundingClientRect();
      return {
        className: element.className,
        ariaLabel: element.getAttribute("aria-label"),
        dataLabel: element.getAttribute("data-label"),
        title: element.getAttribute("title"),
        pressed: element.getAttribute("aria-pressed"),
        disabled: element.hasAttribute("disabled"),
        iconCount: icon ? icon.children.length : 0,
        iconHidden: icon ? icon.getAttribute("aria-hidden") : null,
        iconFocusable: icon ? icon.getAttribute("focusable") : null,
        iconChildren: children,
        text: (element.textContent || "").trim(),
        width: rect.width,
        height: rect.height
      };
    }
    return {
      buttons: Array.from(document.querySelectorAll(".viewer-button")).map(describe),
      original: describe(document.querySelector(".viewer-original"))
    };
  });
}

function controlByClass(controls, className) {
  return controls.buttons.find((button) =>
    String(button.className).split(/\s+/).includes(className)
  );
}

function iconTags(control) {
  return control.iconChildren.map((child) => child.tag).join(" ");
}

function iconPaths(control) {
  return control.iconChildren.map((child) => child.d || "").join(" ");
}

function assertIconControls(label, controls) {
  assert(
    controls.buttons.length === 8,
    label + ": expected 8 viewer buttons, found " + controls.buttons.length
  );
  assert(controls.original, label + ": the original link is missing");

  const all = controls.buttons.concat([controls.original]);
  for (const item of all) {
    const name = item.className || "original link";
    assert(item.iconCount > 0, label + ": " + name + " has no icon");
    assert(item.iconHidden === "true", label + ": " + name + " icon is not aria-hidden");
    assert(item.iconFocusable === "false", label + ": " + name + " icon is focusable");
    assert(item.text === "", label + ": " + name + " still shows visible text");
    assert(
      item.ariaLabel && item.ariaLabel.trim() !== "",
      label + ": " + name + " has no accessible name"
    );
    assert(
      item.dataLabel === item.ariaLabel,
      label + ": " + name + " data-label does not match its accessible name"
    );
    assert(item.title === null, label + ": " + name + " adds a native title");
    assert(
      item.width >= 43.9 && item.height >= 43.9,
      label +
        ": " +
        name +
        " target is " +
        item.width.toFixed(2) +
        "x" +
        item.height.toFixed(2) +
        " px, below 44 px"
    );
  }
}

async function readTooltip(page, selector) {
  return page.evaluate((target) => {
    const style = getComputedStyle(document.querySelector(target), "::after");
    return { content: style.content, visibility: style.visibility };
  }, selector);
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

  const initialControls = await readControls(page);
  assertIconControls("initial", initialControls);

  const initialPlay = controlByClass(initialControls, "viewer-play");
  assert(iconTags(initialPlay) === "polygon", "initial play control is not a play icon");
  assert(initialPlay.ariaLabel === "Play slideshow", "initial play label is wrong");
  assert(initialPlay.pressed === "false", "initial play state is pressed");

  const initialImmersive = controlByClass(initialControls, "viewer-immersive");
  assert(
    iconPaths(initialImmersive).startsWith("M8 3H5"),
    "initial immersive control is not a maximize icon"
  );
  assert(
    initialImmersive.ariaLabel === "Enter immersive mode",
    "initial immersive label is wrong"
  );
  assert(initialImmersive.pressed === "false", "initial immersive state is pressed");

  const hiddenTooltip = await readTooltip(page, ".viewer-next");
  assert(
    hiddenTooltip.content !== "none",
    "the next control has no tooltip content"
  );
  assert(
    hiddenTooltip.visibility === "hidden",
    "the next control tooltip is visible before hover"
  );

  await page.locator(".viewer-next").hover();
  await settle(page);
  const hoverTooltip = await readTooltip(page, ".viewer-next");
  assert(
    hoverTooltip.visibility === "visible",
    "the next control tooltip does not show on hover"
  );
  assert(
    hoverTooltip.content.includes("Next item") ||
      hoverTooltip.content.includes("attr(data-label)"),
    "the next control tooltip is not sourced from data-label"
  );

  await page.keyboard.press("Tab");
  await settle(page);
  const focusedControl = await page.evaluate(() => {
    const active = document.activeElement;
    return active ? active.className : "";
  });
  assert(
    String(focusedControl).includes("viewer-next"),
    "Tab did not move focus to the next control"
  );
  const focusTooltip = await readTooltip(page, ".viewer-next");
  assert(
    focusTooltip.visibility === "visible",
    "the next control tooltip does not show on keyboard focus"
  );

  await page.keyboard.press("s");
  await settle(page);
  const playingControls = await readControls(page);
  assertIconControls("playing", playingControls);
  const playingButton = controlByClass(playingControls, "viewer-play");
  assert(iconTags(playingButton) === "rect rect", "play did not swap to a pause icon");
  assert(playingButton.ariaLabel === "Pause slideshow", "playing label is wrong");
  assert(playingButton.pressed === "true", "playing state is not pressed");

  await page.keyboard.press("s");
  await settle(page);
  const pausedControls = await readControls(page);
  const pausedButton = controlByClass(pausedControls, "viewer-play");
  assert(iconTags(pausedButton) === "polygon", "pause did not swap back to a play icon");
  assert(pausedButton.ariaLabel === "Play slideshow", "paused label is wrong");
  assert(pausedButton.pressed === "false", "paused state is pressed");

  await page.keyboard.press("f");
  await settle(page);
  const immersive = await readGeometry(page);
  assert(immersive && immersive.immersive, "immersive mode did not activate");

  const immersiveControls = await readControls(page);
  assertIconControls("immersive", immersiveControls);
  const immersiveButton = controlByClass(immersiveControls, "viewer-immersive");
  assert(
    iconPaths(immersiveButton).startsWith("M8 3v3"),
    "immersive did not swap to a minimize icon"
  );
  assert(
    immersiveButton.ariaLabel === "Exit immersive mode",
    "immersive label is wrong"
  );
  assert(immersiveButton.pressed === "true", "immersive state is not pressed");

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

  const framedControls = await readControls(page);
  const framedImmersive = controlByClass(framedControls, "viewer-immersive");
  assert(
    iconPaths(framedImmersive).startsWith("M8 3H5"),
    "leaving immersive did not restore the maximize icon"
  );
  assert(
    framedImmersive.ariaLabel === "Enter immersive mode",
    "leaving immersive did not restore the control label"
  );
  assert(framedImmersive.pressed === "false", "leaving immersive kept the pressed state");

  await page.setViewportSize({ width: 390, height: 844 });
  await settle(page);
  const phoneControls = await readControls(page);
  assertIconControls("phone", phoneControls);

  const edgeAlignment = await page.evaluate(() => {
    const viewer = document.querySelector(".viewer").getBoundingClientRect();
    const firstButton = document.querySelector(".viewer-controls .viewer-button");
    const lastControl = document.querySelector(".viewer-original");
    return {
      firstLeft: getComputedStyle(firstButton, "::after").left,
      lastRight: getComputedStyle(lastControl, "::after").right,
      firstRect: firstButton.getBoundingClientRect().left,
      lastRect: lastControl.getBoundingClientRect().right,
      viewerLeft: viewer.left,
      viewerRight: viewer.right
    };
  });
  assert(
    edgeAlignment.firstLeft === "0px",
    "the first control tooltip is not aligned inward"
  );
  assert(
    edgeAlignment.lastRight === "0px",
    "the last control tooltip is not aligned inward"
  );
  assert(
    edgeAlignment.firstRect >= edgeAlignment.viewerLeft &&
      edgeAlignment.lastRect <= edgeAlignment.viewerRight,
    "an edge control sits outside the phone-sized viewer"
  );

  await page.setViewportSize({ width: 1280, height: 800 });
  await page.keyboard.press("Escape");
  await page.locator(".viewer[open]").waitFor({ state: "detached" });

  await page.evaluate(() => {
    const anchor = document.createElement("a");
    anchor.className = "tile-link";
    anchor.href = "/media/fixture/clip.mp4";
    anchor.setAttribute("data-item-path", "fixture/clip.mp4");
    anchor.setAttribute("data-kind", "video");
    anchor.setAttribute("data-captured", "2024-05-03 09:00:00");
    anchor.setAttribute("data-size", "1500000");
    anchor.textContent = "clip.mp4";
    document.querySelector(".grid").appendChild(anchor);
  });

  await page.locator("a.tile-link").last().click();
  await page.locator(".viewer-video").waitFor({ state: "visible" });
  await settle(page);

  const videoControls = await readControls(page);
  assertIconControls("video", videoControls);
  for (const className of ["viewer-zoom-out", "viewer-zoom-in", "viewer-reset"]) {
    const zoomControl = controlByClass(videoControls, className);
    assert(
      zoomControl.disabled,
      "video album: " + className + " should be disabled"
    );
    assert(
      zoomControl.iconCount > 0,
      "video album: " + className + " lost its icon while disabled"
    );
  }
  const videoPrev = controlByClass(videoControls, "viewer-prev");
  const videoNext = controlByClass(videoControls, "viewer-next");
  assert(!videoPrev.disabled, "video album: previous control should be enabled");
  assert(videoNext.disabled, "video album: next control should be disabled at the last item");
  assert(
    videoControls.original.ariaLabel === "Open original: clip.mp4",
    "video album: original link label is wrong"
  );

  console.log("viewer browser regression passed");
  await context.close();
} catch (error) {
  console.error("viewer browser regression failed: " + (error && error.stack ? error.stack : error));
  process.exitCode = 1;
} finally {
  await browser.close();
}
