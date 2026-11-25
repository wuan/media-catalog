import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const TOLERANCE = 1.5;
const SKIP_CODE = 77;
// A real 1x1 JPEG so the browser regression has a decodable original; geometry
// comes from the fixture's catalog dimensions, not from the file.
const LANDSCAPE_ORIGINAL_JPEG =
  "/9j/4AAQSkZJRgABAQEAYABgAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wAALCAABAAEBAREA/8QAFAABAAAAAAAAAAAAAAAAAAAACf/EABQQAQAAAAAAAAAAAAAAAAAAAAD/2gAIAQEAAD8AKp//2Q==";
const LANDSCAPE_ORIGINAL_PATH = "/media/fixture/landscape.jpg";

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
  const fixtureHtml = fs.readFileSync(path.join(root, "tests/js/fixtures/viewer-browser.html"), "utf8");
  const searchHtml = `<!doctype html>
<html lang="en">
  <head><meta charset="utf-8"><title>Search fixture</title>
    <link rel="stylesheet" href="/assets/site.css"></head>
  <body>
    <main>
      <h1>Search</h1>
      <form id="filters" autocomplete="off">
        <label>Text<input id="q" name="q" type="search"></label>
        <label>Type<select id="kind" name="kind"><option value="">All types</option></select></label>
        <label>Year<select id="year" name="year"><option value="">All years</option></select></label>
        <label>Month<select id="month" name="month"><option value="">All months</option></select></label>
      </form>
      <p id="status" class="status" role="status">Loading catalog…</p>
      <div id="results" class="grid"></div>
    </main>
    <script src="/assets/search.js"></script>
  </body>
</html>`;
  const searchIndex = JSON.stringify({
    schema_version: 2,
    total_items: 0,
    location_preview: true,
    years: []
  });
  const files = {
    "/": { body: fixtureHtml, type: "text/html; charset=utf-8" },
    "/preview.html": {
      body: fixtureHtml.replace("<body>", '<body data-location-preview="true">'),
      type: "text/html; charset=utf-8"
    },
    "/tests/js/fixtures/viewer-browser.html": {
      body: fixtureHtml,
      type: "text/html; charset=utf-8"
    },
    "/slideshow.html": {
      body: fs.readFileSync(
        path.join(root, "tests/js/fixtures/viewer-slideshow.html"),
        "utf8"
      ),
      type: "text/html; charset=utf-8"
    },
    "/search.html": { body: searchHtml, type: "text/html; charset=utf-8" },
    "/catalog/index.json": {
      body: searchIndex,
      type: "application/json; charset=utf-8"
    },
    "/assets/site.css": {
      body: fs.readFileSync(path.join(root, "assets/site.css"), "utf8"),
      type: "text/css; charset=utf-8"
    },
    "/assets/viewer.js": {
      body: fs.readFileSync(path.join(root, "assets/viewer.js"), "utf8"),
      type: "text/javascript; charset=utf-8"
    },
    "/assets/search.js": {
      body: fs.readFileSync(path.join(root, "assets/search.js"), "utf8"),
      type: "text/javascript; charset=utf-8"
    }
  };
  const routes = new Map();
  for (const [routePath, entry] of Object.entries(files)) {
    routes.set(routePath, { type: entry.type, body: entry.body });
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

async function waitForCondition(check, timeout, message) {
  const start = Date.now();
  while (!check()) {
    if (Date.now() - start > timeout) {
      throw new Error(message);
    }
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
}

async function readSlideshow(page) {
  return page.evaluate(() => {
    const image = document.querySelector(".viewer-image");
    const play = document.querySelector(".viewer-play");
    const error = document.querySelector(".viewer-error");
    const original = document.querySelector(".viewer-original");
    return {
      position: (document.querySelector(".viewer-position") || {}).textContent || "",
      caption: (document.querySelector(".viewer-captured") || {}).textContent || "",
      src: image ? image.getAttribute("src") : null,
      placeholder: image ? image.classList.contains("is-placeholder") : false,
      original: image ? image.classList.contains("is-original") : false,
      imageCount: document.querySelectorAll(".viewer-image").length,
      placeholders: document.querySelectorAll(".viewer-image.is-placeholder").length,
      search: window.location.search,
      playing: play ? play.getAttribute("aria-pressed") === "true" : false,
      errorHidden: error
        ? error.hidden || getComputedStyle(error).display === "none"
        : true,
      errorText: error ? (error.textContent || "").trim() : "",
      originalHref: original ? original.getAttribute("href") : null
    };
  });
}

// Hold HTMLImageElement.decode() for one URL so a test can observe the viewer
// while the original is downloadable but not yet paintable.
async function installDecodeDelay(page, targetPath) {
  await page.addInitScript((target) => {
    const nativeDecode = HTMLImageElement.prototype.decode;
    const waiters = [];
    window.__decodeRequests = 0;
    window.__decodeCompleted = 0;
    window.__decodeRelease = () => {
      const pending = waiters.splice(0);
      for (const release of pending) {
        release();
      }
    };
    HTMLImageElement.prototype.decode = function () {
      let path = null;
      try {
        path = new URL(this.src, document.baseURI).pathname;
      } catch (ignored) {
        path = null;
      }
      if (path !== target) {
        return nativeDecode.call(this);
      }
      window.__decodeRequests += 1;
      return new Promise((resolve, reject) => {
        waiters.push(() => {
          nativeDecode.call(this).then(
            (value) => {
              window.__decodeCompleted += 1;
              resolve(value);
            },
            (error) => {
              window.__decodeCompleted += 1;
              reject(error);
            }
          );
        });
      });
    };
  }, targetPath);
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

async function readStatusRow(page) {
  return page.evaluate(() => {
    const status = document.querySelector(".viewer-status");
    if (!status) {
      return null;
    }
    const row = status.getBoundingClientRect();

    function describe(selector) {
      const element = document.querySelector(selector);
      if (!element) {
        return null;
      }
      const hidden =
        element.hidden || getComputedStyle(element).display === "none";
      const rect = element.getBoundingClientRect();
      return {
        hidden,
        text: (element.textContent || "").trim(),
        left: rect.left,
        right: rect.right,
        top: rect.top,
        bottom: rect.bottom,
        centerX: rect.left + rect.width / 2,
        width: rect.width,
        height: rect.height
      };
    }

    return {
      row: {
        left: row.left,
        right: row.right,
        top: row.top,
        bottom: row.bottom,
        centerX: row.left + row.width / 2,
        width: row.width,
        height: row.height
      },
      order: Array.from(status.children).map((child) => child.className),
      caption: describe(".viewer-captured"),
      location: describe(".viewer-location"),
      meta: describe(".viewer-meta"),
      statusText: (status.textContent || "").trim(),
      overflow: status.scrollWidth - status.clientWidth
    };
  });
}

function visibleSegments(status) {
  return [status.caption, status.location, status.meta].filter(
    (segment) => segment && !segment.hidden
  );
}

function assertSameRow(label, status) {
  const visible = visibleSegments(status);
  assert(visible.length > 0, label + ": the status row has no visible segments");
  const tallest = Math.max(...visible.map((segment) => segment.height));
  const highestTop = Math.max(...visible.map((segment) => segment.top));
  const lowestBottom = Math.min(...visible.map((segment) => segment.bottom));
  assert(
    highestTop < lowestBottom,
    label + ": status segments do not share one row"
  );
  assert(
    status.row.height <= tallest + 4,
    label +
      ": status row is " +
      status.row.height.toFixed(2) +
      " px tall instead of one line of text"
  );
}

async function readDialogAccessibility(page) {
  return page.evaluate(() => {
    const dialog = document.querySelector(".viewer[open]");
    if (!dialog) {
      return null;
    }
    const labelledby = dialog.getAttribute("aria-labelledby");
    const describedby = dialog.getAttribute("aria-describedby");
    const nameNode = labelledby ? document.getElementById(labelledby) : null;
    const descriptionNode = describedby
      ? document.getElementById(describedby)
      : null;
    return {
      labelledby,
      describedby,
      name: nameNode ? (nameNode.textContent || "").trim() : null,
      description: descriptionNode
        ? (descriptionNode.textContent || "").trim()
        : null
    };
  });
}

async function readPreview(page) {
  return page.evaluate(() => {
    const preview = document.querySelector(".viewer-map-preview");
    const frame = document.querySelector(".viewer-map-frame");
    const caption = document.querySelector(".viewer-map-caption");
    const link = document.querySelector(".viewer-location a");
    const dialog = document.querySelector(".viewer[open]");
    if (!preview || !frame || !caption || !link || !dialog) {
      return null;
    }
    const box = preview.getBoundingClientRect();
    const bounds = dialog.getBoundingClientRect();
    const anchor = link.getBoundingClientRect();
    return {
      visible: !preview.hidden && getComputedStyle(preview).display !== "none",
      left: box.left,
      top: box.top,
      right: box.right,
      bottom: box.bottom,
      width: box.width,
      height: box.height,
      dialog: {
        left: bounds.left,
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.bottom
      },
      anchor: {
        left: anchor.left,
        top: anchor.top,
        right: anchor.right,
        bottom: anchor.bottom
      },
      caption: (caption.textContent || "").trim(),
      title: frame.getAttribute("title"),
      loading: frame.getAttribute("loading"),
      referrerpolicy: frame.getAttribute("referrerpolicy"),
      tabindex: frame.getAttribute("tabindex"),
      src: frame.getAttribute("src"),
      linkHref: link.getAttribute("href"),
      linkTarget: link.getAttribute("target"),
      linkText: (link.textContent || "").trim(),
      active: document.activeElement
        ? document.activeElement.tagName.toLowerCase() +
          "." +
          String(document.activeElement.className || "")
        : null
    };
  });
}

function assertPreviewInside(label, preview) {
  assert(preview && preview.visible, label + ": the map preview is not visible");
  assert(
    preview.left >= preview.dialog.left - TOLERANCE &&
      preview.right <= preview.dialog.right + TOLERANCE &&
      preview.top >= preview.dialog.top - TOLERANCE &&
      preview.bottom <= preview.dialog.bottom + TOLERANCE,
    label +
      ": the map preview is outside the dialog (" +
      preview.left.toFixed(1) +
      "," +
      preview.top.toFixed(1) +
      " to " +
      preview.right.toFixed(1) +
      "," +
      preview.bottom.toFixed(1) +
      ")"
  );
  assert(
    preview.title && preview.title.includes(preview.caption),
    label + ": the preview frame has no coordinate-naming title"
  );
  assert(
    preview.loading === "lazy",
    label + ": the preview frame is not lazy"
  );
  assert(
    preview.referrerpolicy === "no-referrer",
    label + ": the preview frame does not suppress the referrer"
  );
  assert(
    preview.tabindex === "-1",
    label + ": the preview frame can enter the tab order"
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
  const mapRequests = [];
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const page = await context.newPage();

  await page.route("**/*", (route) => {
    const url = new URL(route.request().url());
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      return route.continue();
    }
    if (url.hostname === "openstreetmap.org" || url.hostname.endsWith(".openstreetmap.org")) {
      mapRequests.push(url.href);
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
  assert(
    assetsLoaded.tiles === 5,
    "the fixture should provide five tiles"
  );

  await page.locator("a.tile-link").first().click();
  await page.locator(".viewer[open]").waitFor({ state: "visible" });
  await page.locator(".viewer-image.is-placeholder").waitFor();
  await settle(page);
  assertPreview("initial landscape", await readGeometry(page), landscape);

  const initialStatus = await readStatusRow(page);
  assert(initialStatus, "the viewer did not render a .viewer-status row");
  assert(
    initialStatus.order.join(",") ===
      "viewer-captured,viewer-location,viewer-meta",
    "the status row DOM order is not captured, location, meta"
  );
  assert(
    initialStatus.caption && !initialStatus.caption.hidden,
    "the capture caption is missing from the status row"
  );
  assert(
    initialStatus.location && !initialStatus.location.hidden,
    "the location segment is missing from the status row"
  );
  assert(
    initialStatus.meta && !initialStatus.meta.hidden,
    "the camera settings segment is missing from the status row"
  );
  assert(
    initialStatus.caption.text ===
      "2024-05-01 10:00:00 · Test Camera · 4000x3000 12.0MP · 3.4 MB",
    "the status row caption text is wrong: " + initialStatus.caption.text
  );
  assert(
    initialStatus.location.text === "52.5200, 13.4050",
    "the status row location text is wrong: " + initialStatus.location.text
  );
  assert(
    initialStatus.meta.text === "26 mm · f/1.8 · 1/125 s · ISO 100",
    "the status row settings text is wrong: " + initialStatus.meta.text
  );
  assertSameRow("initial landscape", initialStatus);

  const captionLeftGap = Math.abs(
    initialStatus.caption.left - initialStatus.row.left
  );
  assert(
    captionLeftGap <= TOLERANCE,
    "the caption does not start at the status row's left edge (" +
      captionLeftGap.toFixed(2) +
      " px)"
  );
  const locationCentreGap = Math.abs(
    initialStatus.location.centerX - initialStatus.row.centerX
  );
  assert(
    locationCentreGap <= TOLERANCE,
    "the location's centre is " +
      locationCentreGap.toFixed(2) +
      " px from the status row's centre"
  );
  const settingsRightGap = Math.abs(
    initialStatus.meta.right - initialStatus.row.right
  );
  assert(
    settingsRightGap <= TOLERANCE,
    "the settings do not end at the status row's right edge (" +
      settingsRightGap.toFixed(2) +
      " px)"
  );
  assert(
    initialStatus.overflow <= TOLERANCE,
    "the status row overflows horizontally by " +
      initialStatus.overflow +
      " px"
  );

  const initialAccessibility = await readDialogAccessibility(page);
  assert(
    initialAccessibility && initialAccessibility.labelledby === "viewer-captured",
    "the dialog is not labelled by the capture caption"
  );
  assert(
    initialAccessibility.name === initialStatus.caption.text,
    "the dialog's accessible name is not only the capture caption"
  );
  assert(
    !initialAccessibility.name.includes(initialStatus.meta.text),
    "the dialog's accessible name includes the camera settings"
  );
  assert(
    !initialAccessibility.name.includes(initialStatus.location.text),
    "the dialog's accessible name includes the location"
  );
  assert(
    initialAccessibility.describedby === "viewer-caption",
    "the dialog is not described by the caption element"
  );
  assert(
    initialAccessibility.description.includes(initialStatus.meta.text),
    "the dialog description omits the camera settings"
  );
  assert(
    initialAccessibility.description.includes(initialStatus.location.text),
    "the dialog description omits the location"
  );

  const locationLink = await page.evaluate(() => {
    const link = document.querySelector(".viewer-location a");
    if (!link) {
      return null;
    }
    return {
      href: link.getAttribute("href"),
      label: link.getAttribute("aria-label"),
      target: link.getAttribute("target"),
      rel: link.getAttribute("rel"),
      referrerPolicy: link.getAttribute("referrerpolicy"),
      text: (link.textContent || "").trim()
    };
  });
  assert(locationLink, "the status row has no location link");
  assert(
    locationLink.text === "52.5200, 13.4050",
    "the location link text is wrong: " + locationLink.text
  );
  assert(
    locationLink.href &&
      locationLink.href.includes("openstreetmap.org") &&
      locationLink.href.includes("mlat=52.520008") &&
      locationLink.href.includes("mlon=13.404954"),
    "the location link does not target the item's OpenStreetMap point"
  );
  assert(
    locationLink.label &&
      locationLink.label.includes("52.5200, 13.4050") &&
      locationLink.label.includes("OpenStreetMap") &&
      locationLink.label.includes("new tab"),
    "the location link's accessible name is incomplete: " + locationLink.label
  );
  assert(
    locationLink.target === "_blank",
    "the location link does not open in a new tab"
  );
  assert(
    locationLink.rel === "noopener noreferrer" &&
      locationLink.referrerPolicy === "no-referrer",
    "the location link is not isolated from the catalog window"
  );
  assert(
    mapRequests.length === 0,
    "a map request occurred before the location link was activated: " +
      mapRequests.join(", ")
  );

  const previewAttribute = await page.evaluate(() => ({
    body: document.body.getAttribute("data-location-preview")
  }));
  assert(
    previewAttribute.body === null,
    "the default-off fixture advertises a location preview"
  );
  await page.locator(".viewer-location a").hover();
  await page.evaluate(() => document.querySelector(".viewer-location a").focus());
  await page.waitForTimeout(350);
  const disabledPreview = await readPreview(page);
  assert(
    disabledPreview && !disabledPreview.visible,
    "the default-off viewer revealed a map preview"
  );
  assert(
    mapRequests.length === 0,
    "the default-off viewer requested a map: " + mapRequests.join(", ")
  );
  await page.evaluate(() => document.querySelector(".viewer-close").focus());

  await page.setViewportSize({ width: 375, height: 800 });
  await settle(page);
  const narrowStatus = await readStatusRow(page);
  assert(narrowStatus, "the viewer dropped the status row at a narrow viewport");
  assert(
    narrowStatus.caption && !narrowStatus.caption.hidden,
    "the narrow status row lost the capture caption"
  );
  assert(
    narrowStatus.location && !narrowStatus.location.hidden,
    "the narrow status row lost the location segment"
  );
  assert(
    narrowStatus.meta && !narrowStatus.meta.hidden,
    "the narrow status row lost the settings segment"
  );
  assert(
    narrowStatus.caption.top < narrowStatus.location.top &&
      narrowStatus.location.top < narrowStatus.meta.top,
    "the narrow status row does not stack caption, location, settings"
  );
  assert(
    narrowStatus.caption.bottom <= narrowStatus.location.top + TOLERANCE &&
      narrowStatus.location.bottom <= narrowStatus.meta.top + TOLERANCE,
    "the narrow status row segments overlap"
  );
  assert(
    Math.abs(narrowStatus.caption.left - narrowStatus.row.left) <= TOLERANCE &&
      Math.abs(narrowStatus.location.left - narrowStatus.row.left) <=
        TOLERANCE &&
      Math.abs(narrowStatus.meta.left - narrowStatus.row.left) <= TOLERANCE,
    "the narrow status row segments do not align left"
  );
  assert(
    narrowStatus.overflow <= TOLERANCE,
    "the narrow status row overflows horizontally by " +
      narrowStatus.overflow +
      " px"
  );
  const narrowViewer = await page.evaluate(() => {
    const viewer = document.querySelector(".viewer").getBoundingClientRect();
    return {
      left: viewer.left,
      right: viewer.right,
      innerWidth: window.innerWidth
    };
  });
  assert(
    narrowViewer.left >= -TOLERANCE &&
      narrowViewer.right <= narrowViewer.innerWidth + TOLERANCE,
    "the narrow viewer extends beyond the viewport"
  );
  const narrowAccessibility = await readDialogAccessibility(page);
  assert(
    narrowAccessibility.name === narrowStatus.caption.text,
    "the narrow status row no longer provides the dialog's accessible name"
  );
  await page.setViewportSize({ width: 1280, height: 800 });
  await settle(page);

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

  await page.setViewportSize({ width: 1280, height: 800 });
  await settle(page);

  await page.keyboard.press("ArrowRight");
  await settle(page);
  const captionOnlyStatus = await readStatusRow(page);
  assert(captionOnlyStatus, "the caption-only item did not render a status row");
  assert(
    captionOnlyStatus.caption &&
      !captionOnlyStatus.caption.hidden &&
      captionOnlyStatus.caption.text ===
        "2024-05-03 08:00:00 · Test Camera · 4000x3000 12.0MP · 1.2 MB",
    "the caption-only item lost or changed its caption"
  );
  assert(
    captionOnlyStatus.location && captionOnlyStatus.location.hidden,
    "the caption-only item renders an empty location segment"
  );
  assert(
    captionOnlyStatus.meta && captionOnlyStatus.meta.hidden,
    "the caption-only item renders an empty settings segment"
  );
  assert(
    visibleSegments(captionOnlyStatus).length === 1,
    "the caption-only item renders more than one status segment"
  );
  assert(
    captionOnlyStatus.statusText === captionOnlyStatus.caption.text,
    "the caption-only status row contains extra text"
  );
  assert(
    Math.abs(captionOnlyStatus.caption.left - captionOnlyStatus.row.left) <=
      TOLERANCE,
    "the caption-only item moved its caption off the left edge"
  );
  assertSameRow("caption-only", captionOnlyStatus);
  assert(
    captionOnlyStatus.overflow <= TOLERANCE,
    "the caption-only status row overflows horizontally"
  );

  await page.keyboard.press("ArrowRight");
  await settle(page);
  const settingsOnlyStatus = await readStatusRow(page);
  assert(
    settingsOnlyStatus,
    "the settings-only item did not render a status row"
  );
  assert(
    settingsOnlyStatus.caption && !settingsOnlyStatus.caption.hidden,
    "the settings-only item lost its caption"
  );
  assert(
    settingsOnlyStatus.location && settingsOnlyStatus.location.hidden,
    "the settings-only item renders an empty centre segment"
  );
  assert(
    settingsOnlyStatus.meta && !settingsOnlyStatus.meta.hidden,
    "the settings-only item lost its settings segment"
  );
  assert(
    settingsOnlyStatus.meta.text === "35 mm · f/4 · 1/60 s · ISO 400",
    "the settings-only item settings text is wrong: " +
      settingsOnlyStatus.meta.text
  );
  assert(
    Math.abs(settingsOnlyStatus.caption.left - settingsOnlyStatus.row.left) <=
      TOLERANCE,
    "the settings-only item moved its caption off the left edge"
  );
  assert(
    Math.abs(settingsOnlyStatus.meta.right - settingsOnlyStatus.row.right) <=
      TOLERANCE,
    "the settings-only item moved its settings off the right edge"
  );
  assert(
    visibleSegments(settingsOnlyStatus).length === 2,
    "the settings-only item renders an unexpected number of segments"
  );
  assertSameRow("settings-only", settingsOnlyStatus);

  await page.keyboard.press("ArrowRight");
  await settle(page);
  const coordsOnlyStatus = await readStatusRow(page);
  assert(
    coordsOnlyStatus,
    "the coords-only item did not render a status row"
  );
  assert(
    coordsOnlyStatus.caption && !coordsOnlyStatus.caption.hidden,
    "the coords-only item lost its caption"
  );
  assert(
    coordsOnlyStatus.location && !coordsOnlyStatus.location.hidden,
    "the coords-only item lost its location segment"
  );
  assert(
    coordsOnlyStatus.meta && coordsOnlyStatus.meta.hidden,
    "the coords-only item renders an empty right segment"
  );
  assert(
    coordsOnlyStatus.location.text === "40.6892, -74.0445",
    "the coords-only item location text is wrong: " +
      coordsOnlyStatus.location.text
  );
  assert(
    Math.abs(
      coordsOnlyStatus.location.centerX - coordsOnlyStatus.row.centerX
    ) <= TOLERANCE,
    "the coords-only location is not centred on the status row"
  );
  assert(
    Math.abs(coordsOnlyStatus.caption.left - coordsOnlyStatus.row.left) <=
      TOLERANCE,
    "the coords-only item moved its caption off the left edge"
  );
  assert(
    visibleSegments(coordsOnlyStatus).length === 2,
    "the coords-only item renders an unexpected number of segments"
  );
  assertSameRow("coords-only", coordsOnlyStatus);
  const coordsOnlyLink = await page.evaluate(() => {
    const link = document.querySelector(".viewer-location a");
    return link ? link.getAttribute("aria-label") : null;
  });
  assert(
    coordsOnlyLink &&
      coordsOnlyLink.includes("40.6892, -74.0445") &&
      coordsOnlyLink.includes("OpenStreetMap") &&
      coordsOnlyLink.includes("new tab"),
    "the coords-only location link's accessible name is incomplete: " +
      coordsOnlyLink
  );
  assert(
    mapRequests.length === 0,
    "a map request occurred before the location link was activated: " +
      mapRequests.join(", ")
  );

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

  await context.close();

  async function serveWithRoutes(targetPage, mapLog, shouldBlock, routeTable) {
    const table = routeTable || routes;
    await targetPage.route("**/*", (route) => {
      const url = new URL(route.request().url());
      if (url.protocol !== "http:" && url.protocol !== "https:") {
        return route.continue();
      }
      if (
        url.hostname === "openstreetmap.org" ||
        url.hostname.endsWith(".openstreetmap.org")
      ) {
        mapLog.push(url.href);
        if (shouldBlock()) {
          return route.abort();
        }
        return route.fulfill({
          status: 200,
          contentType: "text/html; charset=utf-8",
          body: "<!doctype html><title>Map</title><p>Map</p>"
        });
      }
      const entry = table.get(url.pathname);
      if (entry) {
        return route.fulfill({
          status: 200,
          contentType: entry.type,
          body: entry.body
        });
      }
      return route.fulfill({
        status: 404,
        contentType: "text/plain; charset=utf-8",
        body: "not found"
      });
    });
  }

  async function serveSlideshowRoutes(targetPage, routeTable, requestLog) {
    await targetPage.route("**/*", (route) => {
      const url = new URL(route.request().url());
      if (url.protocol !== "http:" && url.protocol !== "https:") {
        return route.continue();
      }
      if (url.pathname.startsWith("/media/")) {
        requestLog.push(url.pathname + url.search);
      }
      const entry = routeTable.get(url.pathname);
      if (entry) {
        return route.fulfill({
          status: entry.status || 200,
          contentType: entry.type,
          body: entry.body
        });
      }
      return route.fulfill({
        status: 404,
        contentType: "text/plain; charset=utf-8",
        body: "not found"
      });
    });
  }

  const slideshowRoutes = new Map(routes);
  for (const originalPath of [
    "/media/fixture/landscape.jpg",
    "/media/fixture/portrait.jpg",
    "/media/fixture/caption-only.jpg"
  ]) {
    slideshowRoutes.set(originalPath, {
      type: "image/jpeg",
      body: Buffer.from(LANDSCAPE_ORIGINAL_JPEG, "base64")
    });
  }

  // The search shell learns the setting from the index it loads, before results
  // render, and copies it onto the body for the viewer.
  const searchContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const searchPage = await searchContext.newPage();
  await serveWithRoutes(searchPage, [], () => false);
  await searchPage.goto("http://viewer.test/search.html", { waitUntil: "load" });
  await searchPage.waitForFunction(
    () => document.body.getAttribute("data-location-preview") === "true"
  );
  const propagated = await searchPage.evaluate(() => ({
    attribute: document.body.getAttribute("data-location-preview"),
    status: document.getElementById("status").textContent
  }));
  assert(
    propagated.attribute === "true",
    "the search page did not propagate the index preview setting"
  );
  assert(
    propagated.status !== "Loading catalog…",
    "the search page did not render results after propagating the setting"
  );
  await searchContext.close();

  const previewRequests = [];
  let blockMaps = false;
  const previewContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const previewPage = await previewContext.newPage();
  await serveWithRoutes(previewPage, previewRequests, () => blockMaps);
  await previewPage.goto("http://viewer.test/preview.html", { waitUntil: "load" });
  await previewPage.locator("a.tile-link").first().click();
  await previewPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await settle(previewPage);

  const beforeIntent = await readPreview(previewPage);
  assert(
    beforeIntent && !beforeIntent.visible,
    "the enabled fixture showed a preview before intent"
  );
  assert(
    previewRequests.length === 0,
    "the enabled fixture requested a map before intent: " +
      previewRequests.join(", ")
  );

  await previewPage.locator(".viewer-location a").hover();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await settle(previewPage);
  const hovered = await readPreview(previewPage);
  assertPreviewInside("hover preview", hovered);
  assert(
    hovered.caption === "52.5200, 13.4050",
    "the preview caption is wrong: " + hovered.caption
  );
  assert(
    hovered.bottom <= hovered.anchor.top + TOLERANCE,
    "the preview is not placed above the location link"
  );
  assert(
    previewRequests.length === 1,
    "hover produced " + previewRequests.length + " map requests instead of one"
  );
  const requestUrl = decodeURIComponent(previewRequests[0]);
  assert(
    requestUrl.includes("/export/embed.html"),
    "the preview does not use the official OpenStreetMap embed: " + requestUrl
  );
  assert(
    requestUrl.includes("bbox=13.400024,52.517008,13.409884,52.523008"),
    "the preview request has no bbox around the point: " + requestUrl
  );
  assert(
    requestUrl.includes("marker=52.520008,13.404954"),
    "the preview request has no marker at the point: " + requestUrl
  );

  await previewPage.mouse.move(4, 4);
  await settle(previewPage);
  const leftPreview = await readPreview(previewPage);
  assert(
    leftPreview && !leftPreview.visible,
    "the preview did not hide when the pointer left"
  );

  await previewPage.locator(".viewer-location a").hover();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  assert(
    previewRequests.length === 1,
    "a second reveal requested the map again: " + previewRequests.join(", ")
  );

  await previewPage.mouse.move(4, 4);
  await settle(previewPage);
  await previewPage.evaluate(() =>
    document.querySelector(".viewer-location a").focus()
  );
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  assert(
    previewRequests.length === 1,
    "keyboard focus requested the map a second time: " +
      previewRequests.join(", ")
  );

  await previewPage.keyboard.press("Tab");
  await settle(previewPage);
  const tabbed = await readPreview(previewPage);
  assert(
    tabbed &&
      tabbed.active &&
      tabbed.active.startsWith("button.") &&
      tabbed.active.includes("viewer-"),
    "Tab entered the preview frame instead of a viewer control: " +
      (tabbed && tabbed.active)
  );

  const previewAccessibility = await readDialogAccessibility(previewPage);
  const previewLink = await previewPage.evaluate(() => {
    const link = document.querySelector(".viewer-location a");
    return link
      ? {
          label: link.getAttribute("aria-label"),
          text: (link.textContent || "").trim(),
          target: link.getAttribute("target")
        }
      : null;
  });
  assert(
    previewAccessibility &&
      previewAccessibility.describedby === "viewer-caption" &&
      previewAccessibility.description.includes("52.5200, 13.4050"),
    "the preview changed the dialog description"
  );
  assert(
    previewLink &&
      previewLink.text === "52.5200, 13.4050" &&
      previewLink.label.includes("52.5200, 13.4050") &&
      previewLink.label.includes("OpenStreetMap") &&
      previewLink.label.includes("new tab") &&
      previewLink.target === "_blank",
    "the preview changed the location link's name or target"
  );

  await previewPage.locator(".viewer-location a").focus();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await previewPage.keyboard.press("Escape");
  await settle(previewPage);
  const escapedPreview = await readPreview(previewPage);
  assert(
    escapedPreview && !escapedPreview.visible,
    "Escape did not hide the preview"
  );
  assert(
    (await previewPage.locator(".viewer[open]").count()) === 1,
    "Escape closed the viewer instead of dismissing the preview"
  );

  await previewPage.evaluate(() => {
    const link = document.querySelector(".viewer-location a");
    link.blur();
    link.focus();
  });
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await previewPage.keyboard.press("ArrowRight");
  await settle(previewPage);
  const changedItemPreview = await readPreview(previewPage);
  assert(
    changedItemPreview && !changedItemPreview.visible,
    "the preview stayed visible after the item changed"
  );
  assert(
    changedItemPreview.caption !== "52.5200, 13.4050",
    "the item change did not reach the next item"
  );

  blockMaps = true;
  await previewPage.locator(".viewer-location a").hover();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await settle(previewPage);
  const blockedPreview = await readPreview(previewPage);
  assertPreviewInside("blocked embed", blockedPreview);
  assert(
    blockedPreview.caption === "48.8584, 2.2945",
    "the blocked embed lost its coordinate fallback: " +
      blockedPreview.caption
  );
  assert(
    blockedPreview.linkHref &&
      blockedPreview.linkHref.includes("mlat=48.85837") &&
      blockedPreview.linkHref.includes("mlon=2.294481"),
    "the blocked embed changed the location link: " + blockedPreview.linkHref
  );
  assert(
    blockedPreview.height >= 240 && blockedPreview.height <= 280,
    "the blocked embed changed the popover height: " +
      blockedPreview.height.toFixed(1)
  );

  const requestsAfterBlocked = previewRequests.length;
  await previewPage.locator(".viewer-next").click();
  await settle(previewPage);
  const noCoordsPreview = await readPreview(previewPage);
  assert(
    noCoordsPreview && !noCoordsPreview.visible,
    "an item without coordinates revealed a preview"
  );
  assert(
    noCoordsPreview.linkText === "",
    "an item without coordinates kept the location link text"
  );
  assert(
    previewRequests.length === requestsAfterBlocked,
    "an item without coordinates requested map data"
  );
  await previewPage.locator(".viewer-prev").click();
  await settle(previewPage);

  await previewPage.setViewportSize({ width: 375, height: 800 });
  await settle(previewPage);
  await previewPage.mouse.move(4, 4);
  await previewPage.locator(".viewer-location a").hover();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await settle(previewPage);
  const narrowPreview = await readPreview(previewPage);
  assertPreviewInside("narrow preview", narrowPreview);
  assert(
    narrowPreview.left >= -TOLERANCE &&
      narrowPreview.right <= 375 + TOLERANCE,
    "the narrow preview extends beyond the viewport"
  );

  await previewPage.setViewportSize({ width: 1280, height: 800 });
  await settle(previewPage);
  await previewPage.keyboard.press("f");
  await settle(previewPage);
  await previewPage.mouse.move(4, 4);
  await previewPage.locator(".viewer-location a").hover();
  await previewPage.locator(".viewer-map-preview").waitFor({ state: "visible" });
  await settle(previewPage);
  const immersivePreview = await readPreview(previewPage);
  assertPreviewInside("immersive preview", immersivePreview);
  assert(
    immersivePreview.dialog.left <= TOLERANCE &&
      immersivePreview.dialog.right >= 1280 - TOLERANCE,
    "immersive mode did not fill the viewport"
  );

  await previewPage.keyboard.press("Escape");
  await settle(previewPage);
  await previewPage.keyboard.press("Escape");
  await settle(previewPage);
  await previewPage.keyboard.press("Escape");
  await previewPage.locator(".viewer[open]").waitFor({ state: "detached" });
  await previewContext.close();

  const touchRequests = [];
  const touchContext = await browser.newContext({
    viewport: { width: 390, height: 844 },
    hasTouch: true,
    isMobile: true
  });
  const touchPage = await touchContext.newPage();
  await serveWithRoutes(touchPage, touchRequests, () => false);
  await touchPage.goto("http://viewer.test/preview.html", { waitUntil: "load" });
  await touchPage.locator("a.tile-link").first().click();
  await touchPage.locator(".viewer[open]").waitFor({ state: "visible" });
  const touchMedia = await touchPage.evaluate(() => ({
    hoverNone: window.matchMedia("(hover: none)").matches,
    coarse: window.matchMedia("(pointer: coarse)").matches
  }));
  assert(
    touchMedia.hoverNone || touchMedia.coarse,
    "the touch context does not emulate a coarse pointer"
  );
  await touchPage.evaluate(() => {
    const link = document.querySelector(".viewer-location a");
    link.dispatchEvent(
      new PointerEvent("pointerenter", { pointerType: "touch", bubbles: false })
    );
    link.focus();
  });
  await touchPage.waitForTimeout(350);
  const touchPreview = await readPreview(touchPage);
  assert(
    touchPreview && !touchPreview.visible,
    "touch input revealed a map preview"
  );
  assert(
    touchRequests.length === 0,
    "touch input requested a map: " + touchRequests.join(", ")
  );
  assert(
    touchPreview.linkTarget === "_blank" &&
      touchPreview.linkHref.includes("mlat=52.520008"),
    "touch input changed the location link"
  );
  await touchContext.close();

  // The thumbnail must stay painted while the original downloads and decodes,
  // then the decoded original replaces it in one DOM operation.
  const decodeRoutes = new Map(routes);
  decodeRoutes.set(LANDSCAPE_ORIGINAL_PATH, {
    type: "image/jpeg",
    body: Buffer.from(LANDSCAPE_ORIGINAL_JPEG, "base64")
  });

  const decodeContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const decodePage = await decodeContext.newPage();
  await installDecodeDelay(decodePage, LANDSCAPE_ORIGINAL_PATH);
  await serveWithRoutes(decodePage, [], () => false, decodeRoutes);
  await decodePage.goto("http://viewer.test/", { waitUntil: "load" });
  await decodePage.locator("a.tile-link").first().click();
  await decodePage.locator(".viewer[open]").waitFor({ state: "visible" });
  await decodePage.locator(".viewer-image.is-placeholder").waitFor();
  await decodePage.waitForFunction(() => window.__decodeRequests >= 1);

  const decoding = await decodePage.evaluate(() => {
    const images = Array.from(document.querySelectorAll(".viewer-image"));
    const image = images[0];
    return {
      count: images.length,
      placeholder: image.classList.contains("is-placeholder"),
      original: image.classList.contains("is-original"),
      src: image.getAttribute("src"),
      thumb: document
        .querySelector('a.tile-link[data-item-path="fixture/landscape.jpg"]')
        .getAttribute("data-thumb-url")
    };
  });
  assert(
    decoding.count === 1,
    "the stage held " + decoding.count + " images while the original decoded"
  );
  assert(
    decoding.placeholder && !decoding.original,
    "the viewer dropped the thumbnail while the original was decoding"
  );
  assert(
    decoding.src === decoding.thumb,
    "the visible image stopped pointing at the thumbnail while the original decoded: " +
      decoding.src
  );

  await decodePage.evaluate(() => window.__decodeRelease());
  await decodePage.locator(".viewer-image.is-original").waitFor();
  await settle(decodePage);
  const adopted = await decodePage.evaluate(() => {
    const images = Array.from(document.querySelectorAll(".viewer-image"));
    const image = images[0];
    return {
      count: images.length,
      placeholder: image.classList.contains("is-placeholder"),
      original: image.classList.contains("is-original"),
      src: image.getAttribute("src"),
      alt: image.getAttribute("alt"),
      draggable: image.draggable
    };
  });
  assert(
    adopted.count === 1,
    "the stage held " + adopted.count + " images after adopting the original"
  );
  assert(
    adopted.original && !adopted.placeholder,
    "the decoded original was not adopted as the viewer image"
  );
  assert(
    new URL(adopted.src, "http://viewer.test/").pathname ===
      LANDSCAPE_ORIGINAL_PATH,
    "the adopted image does not show the original: " + adopted.src
  );
  assert(
    adopted.alt === "Landscape fixture",
    "the adopted image lost the item's alt text: " + adopted.alt
  );
  assert(
    adopted.draggable === false,
    "the adopted image became draggable"
  );
  await decodeContext.close();

  // Navigating while the previous original is decoding keeps the new preview
  // and never installs the stale original.
  const navContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const navPage = await navContext.newPage();
  await installDecodeDelay(navPage, LANDSCAPE_ORIGINAL_PATH);
  await serveWithRoutes(navPage, [], () => false, decodeRoutes);
  await navPage.goto("http://viewer.test/", { waitUntil: "load" });
  await navPage.locator("a.tile-link").first().click();
  await navPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await navPage.locator(".viewer-image.is-placeholder").waitFor();
  await navPage.waitForFunction(() => window.__decodeRequests >= 1);

  await navPage.keyboard.press("ArrowRight");
  await settle(navPage);
  await navPage.evaluate(() => window.__decodeRelease());
  await navPage.waitForFunction(() => window.__decodeCompleted >= 1);
  await settle(navPage);

  const afterNavigation = await navPage.evaluate(() => {
    const images = Array.from(document.querySelectorAll(".viewer-image"));
    const image = images[0];
    return {
      count: images.length,
      placeholder: image.classList.contains("is-placeholder"),
      original: image.classList.contains("is-original"),
      src: image.getAttribute("src"),
      thumb: document
        .querySelector('a.tile-link[data-item-path="fixture/portrait.jpg"]')
        .getAttribute("data-thumb-url"),
      staleOriginals: document.querySelectorAll(
        'img.viewer-image[src*="landscape.jpg"]'
      ).length,
      caption: (document.querySelector(".viewer-captured") || {}).textContent || ""
    };
  });
  assert(
    afterNavigation.count === 1,
    "the stage held " +
      afterNavigation.count +
      " images after navigating during decode"
  );
  assert(
    afterNavigation.placeholder && !afterNavigation.original,
    "the stale original replaced the next item's preview"
  );
  assert(
    afterNavigation.src === afterNavigation.thumb,
    "the next item's preview stopped pointing at its thumbnail: " +
      afterNavigation.src
  );
  assert(
    afterNavigation.staleOriginals === 0,
    "the previous item's original was installed after navigation"
  );
  assert(
    afterNavigation.caption.startsWith("2024-05-02"),
    "navigation during decode did not reach the next item: " +
      afterNavigation.caption.trim()
  );
  await navContext.close();

  // Slideshow playback: prepare the next decoded original, skip videos,
  // prefetch the next image, and commit only after the original paints.
  const slideshowRequests = [];
  const slideshowContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const slideshowPage = await slideshowContext.newPage();
  await serveSlideshowRoutes(slideshowPage, slideshowRoutes, slideshowRequests);
  await slideshowPage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await slideshowPage.locator("a.tile-link").first().click();
  await slideshowPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await slideshowPage.locator(".viewer-image.is-placeholder").waitFor();
  await slideshowPage.keyboard.press("s");
  await slideshowPage.locator(".viewer-image.is-original").waitFor();
  await settle(slideshowPage);

  const startedSlideshow = await readSlideshow(slideshowPage);
  assert(
    startedSlideshow.position === "Item 1 of 5",
    "slideshow start did not keep the open image: " + startedSlideshow.position
  );
  assert(
    startedSlideshow.caption.startsWith("2024-05-01"),
    "slideshow start changed the caption: " + startedSlideshow.caption
  );
  assert(
    startedSlideshow.original && !startedSlideshow.placeholder,
    "slideshow start did not paint the decoded original"
  );
  assert(startedSlideshow.playing, "slideshow start did not enter playback");

  await waitForCondition(
    () => slideshowRequests.includes("/media/fixture/portrait.jpg"),
    2000,
    "the slideshow did not prefetch the next image original during the dwell"
  );
  assert(
    slideshowRequests.filter((request) => request === "/media/fixture/portrait.jpg")
      .length === 1,
    "the prefetched next image was requested more than once"
  );

  await slideshowPage.waitForFunction(
    () => document.querySelector(".viewer-position").textContent === "Item 3 of 5",
    null,
    { timeout: 8000 }
  );
  await settle(slideshowPage);
  const advancedSlideshow = await readSlideshow(slideshowPage);
  assert(
    advancedSlideshow.caption.startsWith("2024-05-03"),
    "slideshow advance skipped the wrong items: " + advancedSlideshow.caption
  );
  const portraitRequests = slideshowRequests.filter((request) =>
    request.startsWith("/media/fixture/portrait.jpg")
  );
  assert(
    portraitRequests.length === 1,
    "the commit downloaded the prefetched original again: " +
      portraitRequests.join(", ")
  );
  assert(
    advancedSlideshow.src &&
      new URL(advancedSlideshow.src, "http://viewer.test/").pathname ===
        "/media/fixture/portrait.jpg",
    "the advanced slide is not the decoded original: " + advancedSlideshow.src
  );
  assert(
    advancedSlideshow.original && !advancedSlideshow.placeholder,
    "the advanced slide painted a thumbnail or placeholder"
  );
  assert(
    advancedSlideshow.placeholders === 0,
    "the slideshow stage held a placeholder image"
  );
  assert(
    advancedSlideshow.search.includes("item=fixture%2Fportrait.jpg"),
    "the slideshow advance did not update the album URL: " +
      advancedSlideshow.search
  );
  assert(
    advancedSlideshow.originalHref === "/media/fixture/portrait.jpg",
    "the advanced slide original link is wrong: " + advancedSlideshow.originalHref
  );
  assert(advancedSlideshow.playing, "slideshow stopped after advancing");
  await slideshowContext.close();

  // A decode-delayed original keeps the current image visible until release,
  // then commits the prefetched element without a second download or decode.
  const delayedRequests = [];
  const delayedContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const delayedPage = await delayedContext.newPage();
  await installDecodeDelay(delayedPage, "/media/fixture/portrait.jpg");
  await serveSlideshowRoutes(delayedPage, slideshowRoutes, delayedRequests);
  await delayedPage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await delayedPage.locator("a.tile-link").first().click();
  await delayedPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await delayedPage.locator(".viewer-image.is-placeholder").waitFor();
  await delayedPage.keyboard.press("s");
  await delayedPage.locator(".viewer-image.is-original").waitFor();
  await delayedPage.waitForFunction(() => window.__decodeRequests >= 1);
  await delayedPage.waitForTimeout(5200);

  const heldSlide = await readSlideshow(delayedPage);
  assert(
    heldSlide.position === "Item 1 of 5",
    "the slideshow advanced before the next original decoded: " +
      heldSlide.position
  );
  assert(
    heldSlide.caption.startsWith("2024-05-01"),
    "the held slide caption changed before decode: " + heldSlide.caption
  );
  assert(
    heldSlide.src &&
      new URL(heldSlide.src, "http://viewer.test/").pathname ===
        "/media/fixture/landscape.jpg",
    "the held slide stopped showing the current original: " + heldSlide.src
  );
  assert(
    heldSlide.original && !heldSlide.placeholder && heldSlide.imageCount === 1,
    "the transition painted a thumbnail or blank frame before decode"
  );

  await delayedPage.evaluate(() => window.__decodeRelease());
  await delayedPage.waitForFunction(
    () => document.querySelector(".viewer-position").textContent === "Item 3 of 5",
    null,
    { timeout: 3000 }
  );
  await settle(delayedPage);
  const releasedSlide = await readSlideshow(delayedPage);
  assert(
    releasedSlide.caption.startsWith("2024-05-03"),
    "the released original did not commit the next slide: " + releasedSlide.caption
  );
  assert(
    releasedSlide.original && !releasedSlide.placeholder,
    "the committed slide is not the decoded original"
  );
  const decodeCounts = await delayedPage.evaluate(() => ({
    requests: window.__decodeRequests,
    completed: window.__decodeCompleted
  }));
  assert(
    decodeCounts.requests === 1 && decodeCounts.completed === 1,
    "the commit did not adopt the prefetched decode: " +
      JSON.stringify(decodeCounts)
  );
  await delayedContext.close();

  // Pausing while the next original is still decoding cancels the commit.
  const cancelRequests = [];
  const cancelContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const cancelPage = await cancelContext.newPage();
  await installDecodeDelay(cancelPage, "/media/fixture/portrait.jpg");
  await serveSlideshowRoutes(cancelPage, slideshowRoutes, cancelRequests);
  await cancelPage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await cancelPage.locator("a.tile-link").first().click();
  await cancelPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await cancelPage.locator(".viewer-image.is-placeholder").waitFor();
  await cancelPage.keyboard.press("s");
  await cancelPage.locator(".viewer-image.is-original").waitFor();
  await cancelPage.waitForFunction(() => window.__decodeRequests >= 1);
  await cancelPage.waitForTimeout(5200);

  await cancelPage.keyboard.press("s");
  await settle(cancelPage);
  const pausedPending = await readSlideshow(cancelPage);
  assert(!pausedPending.playing, "the pause control did not stop playback");

  await cancelPage.evaluate(() => window.__decodeRelease());
  await cancelPage.waitForTimeout(300);
  await settle(cancelPage);
  const afterCancel = await readSlideshow(cancelPage);
  assert(
    afterCancel.position === "Item 1 of 5" &&
      afterCancel.caption.startsWith("2024-05-01"),
    "a cancelled commit changed the slide after pause: " + afterCancel.position
  );
  assert(
    afterCancel.src &&
      new URL(afterCancel.src, "http://viewer.test/").pathname ===
        "/media/fixture/landscape.jpg",
    "a stale original replaced the paused slide: " + afterCancel.src
  );
  assert(
    afterCancel.original && !afterCancel.placeholder && afterCancel.imageCount === 1,
    "the cancelled commit painted a thumbnail or blank frame"
  );
  await cancelContext.close();

  // A reduced-data preference skips the prefetch; the advance waits instead.
  const saveDataRequests = [];
  const saveDataContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const saveDataPage = await saveDataContext.newPage();
  await saveDataPage.addInitScript(() => {
    Object.defineProperty(navigator, "connection", {
      value: { saveData: true },
      configurable: true
    });
  });
  await serveSlideshowRoutes(saveDataPage, slideshowRoutes, saveDataRequests);
  await saveDataPage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await saveDataPage.locator("a.tile-link").first().click();
  await saveDataPage.locator(".viewer[open]").waitFor({ state: "visible" });
  await saveDataPage.locator(".viewer-image.is-placeholder").waitFor();
  await saveDataPage.keyboard.press("s");
  await saveDataPage.locator(".viewer-image.is-original").waitFor();
  await saveDataPage.waitForTimeout(500);
  assert(
    !saveDataRequests.some((request) =>
      request.startsWith("/media/fixture/portrait.jpg")
    ),
    "the slideshow prefetched with saveData set: " + saveDataRequests.join(", ")
  );

  await saveDataPage.waitForFunction(
    () => document.querySelector(".viewer-position").textContent === "Item 3 of 5",
    null,
    { timeout: 8000 }
  );
  await settle(saveDataPage);
  const saveDataAdvanced = await readSlideshow(saveDataPage);
  assert(
    saveDataAdvanced.original && !saveDataAdvanced.placeholder,
    "the saveData advance painted a thumbnail or placeholder"
  );
  assert(
    saveDataRequests.filter((request) =>
      request.startsWith("/media/fixture/portrait.jpg")
    ).length === 1,
    "the saveData advance did not request the next original once: " +
      saveDataRequests.join(", ")
  );
  await saveDataContext.close();

  // The retry ladder records three attempts, then playback stops on the
  // failed item with its thumbnail preview and error note.
  const failureRequests = [];
  const failureRoutes = new Map(slideshowRoutes);
  failureRoutes.set("/media/fixture/portrait.jpg", {
    type: "image/jpeg",
    body: Buffer.from("not a jpeg")
  });
  const failureContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const failurePage = await failureContext.newPage();
  await serveSlideshowRoutes(failurePage, failureRoutes, failureRequests);
  await failurePage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await failurePage.locator("a.tile-link").first().click();
  await failurePage.locator(".viewer[open]").waitFor({ state: "visible" });
  await failurePage.locator(".viewer-image.is-placeholder").waitFor();
  await failurePage.keyboard.press("s");
  await failurePage.locator(".viewer-image.is-original").waitFor();
  await failurePage.waitForFunction(
    () => {
      const error = document.querySelector(".viewer-error");
      return (
        error &&
        !error.hidden &&
        getComputedStyle(error).display !== "none"
      );
    },
    null,
    { timeout: 6000 }
  );
  await settle(failurePage);

  const failedSlide = await readSlideshow(failurePage);
  assert(
    failedSlide.position === "Item 3 of 5",
    "the failed slide opened at the wrong item: " + failedSlide.position
  );
  assert(
    !failedSlide.playing,
    "playback kept running after the slide exhausted its retries"
  );
  assert(
    failedSlide.placeholder && !failedSlide.original,
    "the failed item lost its thumbnail preview"
  );
  assert(
    failedSlide.errorText.includes("could not be loaded"),
    "the failed item has no image error note: " + failedSlide.errorText
  );
  assert(
    failedSlide.originalHref === "/media/fixture/portrait.jpg",
    "the failed item original link is wrong: " + failedSlide.originalHref
  );
  assert(
    failedSlide.search.includes("item=fixture%2Fportrait.jpg"),
    "the failed item did not update the album URL: " + failedSlide.search
  );
  const portraitAttempts = failureRequests.filter((request) =>
    request.startsWith("/media/fixture/portrait.jpg")
  );
  assert(
    portraitAttempts.length === 3,
    "the retry ladder made " +
      portraitAttempts.length +
      " requests instead of 3: " +
      portraitAttempts.join(", ")
  );
  assert(
    portraitAttempts[0] === "/media/fixture/portrait.jpg",
    "the first attempt was not the plain original: " + portraitAttempts[0]
  );
  assert(
    portraitAttempts[1] === "/media/fixture/portrait.jpg?slideshow-retry=2",
    "the second attempt has no cache-buster: " + portraitAttempts[1]
  );
  assert(
    portraitAttempts[2] === "/media/fixture/portrait.jpg?slideshow-retry=3",
    "the third attempt has no cache-buster: " + portraitAttempts[2]
  );
  await failurePage.waitForTimeout(5200);
  await settle(failurePage);
  const afterFailureWait = await readSlideshow(failurePage);
  assert(
    afterFailureWait.position === "Item 3 of 5" && !afterFailureWait.playing,
    "a timer advanced the failed slideshow: " + afterFailureWait.position
  );
  await failureContext.close();

  // Starting on a video moves to the next image; starting on the last item
  // restarts from the first image.
  const videoContext = await browser.newContext({
    viewport: { width: 1280, height: 800 }
  });
  const videoPage = await videoContext.newPage();
  await serveSlideshowRoutes(videoPage, slideshowRoutes, []);
  await videoPage.goto("http://viewer.test/slideshow.html", {
    waitUntil: "load"
  });
  await videoPage.locator("a.tile-link").nth(1).click();
  await videoPage.locator(".viewer-video").waitFor({ state: "visible" });
  await videoPage.keyboard.press("s");
  await videoPage.waitForFunction(
    () => document.querySelector(".viewer-position").textContent === "Item 3 of 5",
    null,
    { timeout: 3000 }
  );
  await settle(videoPage);
  const fromVideo = await readSlideshow(videoPage);
  assert(
    fromVideo.caption.startsWith("2024-05-03") && fromVideo.original,
    "starting on a video did not move to the next image: " + fromVideo.caption
  );
  assert(fromVideo.playing, "starting on a video did not begin playback");

  await videoPage.keyboard.press("s");
  await settle(videoPage);
  await videoPage.keyboard.press("Escape");
  await videoPage.locator(".viewer[open]").waitFor({ state: "detached" });

  await videoPage.locator("a.tile-link").last().click();
  await videoPage.locator(".viewer-video").waitFor({ state: "visible" });
  await videoPage.keyboard.press("s");
  await videoPage.waitForFunction(
    () => document.querySelector(".viewer-position").textContent === "Item 1 of 5",
    null,
    { timeout: 3000 }
  );
  await settle(videoPage);
  const restarted = await readSlideshow(videoPage);
  assert(
    restarted.caption.startsWith("2024-05-01") && restarted.original,
    "starting on the last item did not restart from the first image: " +
      restarted.caption
  );
  assert(restarted.playing, "the restart did not begin playback");
  await videoContext.close();

  console.log("viewer browser regression passed");
} catch (error) {
  console.error("viewer browser regression failed: " + (error && error.stack ? error.stack : error));
  process.exitCode = 1;
} finally {
  await browser.close();
}
