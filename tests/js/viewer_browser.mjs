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

  console.log("viewer browser regression passed");
  await context.close();
} catch (error) {
  console.error("viewer browser regression failed: " + (error && error.stack ? error.stack : error));
  process.exitCode = 1;
} finally {
  await browser.close();
}
