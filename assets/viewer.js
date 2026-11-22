(function () {
  "use strict";

  var MIN_ZOOM = 1;
  var MAX_ZOOM = 4;
  var DOUBLE_TAP_ZOOM = 2.5;
  var SWIPE_MIN = 40;
  var SWIPE_RATIO = 0.12;
  var RESULTS_EVENT = "media-catalog:results-rendered";
  // Wait this long on an item before fetching its original, so holding an
  // arrow key to browse only loads thumbnails for the items passed over.
  var ORIGINAL_DELAY_MS = 300;

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function clampZoom(value) {
    return clamp(value, MIN_ZOOM, MAX_ZOOM);
  }

  function parseItemParam(search) {
    var value = new URLSearchParams(search || "").get("item");
    return value ? value : null;
  }

  function buildItemUrl(href, itemPath) {
    var url = new URL(href, "http://localhost");
    if (itemPath) {
      url.searchParams.set("item", itemPath);
    } else {
      url.searchParams.delete("item");
    }
    return url.pathname + url.search + url.hash;
  }

  function albumIndexOf(items, path) {
    for (var index = 0; index < items.length; index += 1) {
      if (items[index].path === path) {
        return index;
      }
    }
    return -1;
  }

  function nextIndex(index, direction, length) {
    var next = index + direction;
    return next < 0 || next >= length ? -1 : next;
  }

  function swipeThreshold(stageWidth) {
    return Math.max(SWIPE_MIN, (stageWidth || 0) * SWIPE_RATIO);
  }

  function isSwipe(dx, dy, stageWidth) {
    return (
      Math.abs(dx) >= swipeThreshold(stageWidth) &&
      Math.abs(dx) >= Math.abs(dy) * 1.5
    );
  }

  function fittedBox(width, height, stageWidth, stageHeight) {
    if (!width || !height || !stageWidth || !stageHeight) {
      return null;
    }
    var fit = Math.min(stageWidth / width, stageHeight / height);
    return { width: width * fit, height: height * fit };
  }

  function placeholderBox(width, height, stageWidth, stageHeight) {
    var box = fittedBox(width, height, stageWidth, stageHeight);
    if (!box) {
      return null;
    }
    // Thumbnails are square center crops of the original, so they cover
    // exactly the original's shorter edge; size and center them on that.
    var edge = Math.min(box.width, box.height);
    return { width: edge, height: edge };
  }

  function clampPan(x, y, scale, baseWidth, baseHeight, stageWidth, stageHeight) {
    var maxX = Math.max(0, ((baseWidth || 0) * scale - stageWidth) / 2);
    var maxY = Math.max(0, ((baseHeight || 0) * scale - stageHeight) / 2);
    return {
      x: clamp(x, -maxX, maxX),
      y: clamp(y, -maxY, maxY)
    };
  }

  function zoomAroundPoint(scale, x, y, factor, point) {
    var nextScale = clampZoom(scale * factor);
    var ratio = nextScale / scale;
    if (nextScale === MIN_ZOOM) {
      return { scale: MIN_ZOOM, x: 0, y: 0 };
    }
    return {
      scale: nextScale,
      x: point.x - (point.x - x) * ratio,
      y: point.y - (point.y - y) * ratio
    };
  }

  function tileFromElement(element) {
    var mediaUrl = element.getAttribute("href") || "";
    var path = element.getAttribute("data-item-path") || "";
    if (!path) {
      try {
        path = decodeURIComponent(mediaUrl.replace(/^\/media\//, ""));
      } catch (ignored) {
        path = mediaUrl.replace(/^\/media\//, "");
      }
    }
    var nameNode = element.querySelector(".name");
    var modelNode = element.querySelector(".model");
    var name = nameNode ? nameNode.textContent : path.split("/").pop();
    var image = element.querySelector("img");
    var placeholder = element.querySelector(".placeholder");
    var alt =
      (image && image.getAttribute("alt")) ||
      (placeholder && placeholder.getAttribute("aria-label")) ||
      name ||
      path;
    var width = Number(element.getAttribute("data-width"));
    var height = Number(element.getAttribute("data-height"));
    return {
      path: path,
      kind: element.getAttribute("data-kind") || "image",
      captured: element.getAttribute("data-captured") || "",
      model: modelNode ? modelNode.textContent.trim() : "",
      thumbUrl: element.getAttribute("data-thumb-url") || "",
      width: width > 0 ? width : null,
      height: height > 0 ? height : null,
      mediaUrl: mediaUrl,
      alt: alt,
      name: name || path,
      element: element
    };
  }

  function collectAlbum(root) {
    var nodes = (root || document).querySelectorAll("a.tile-link");
    var items = [];
    for (var index = 0; index < nodes.length; index += 1) {
      items.push(tileFromElement(nodes[index]));
    }
    return items;
  }

  var state = {
    items: [],
    index: -1,
    opener: null,
    open: false,
    scale: MIN_ZOOM,
    x: 0,
    y: 0,
    baseWidth: 0,
    baseHeight: 0,
    stageWidth: 0,
    stageHeight: 0,
    mediaKind: "image",
    media: null,
    pendingItem: null,
    originalTimer: null,
    originalLoader: null
  };

  var dialog = null;
  var frame = null;
  var stage = null;
  var wrap = null;
  var position = null;
  var captured = null;
  var error = null;
  var prevButton = null;
  var nextButton = null;
  var closeButton = null;
  var originalLink = null;
  var zoomInButton = null;
  var zoomOutButton = null;
  var resetButton = null;
  var pointers = new Map();
  var gesture = null;
  var lastTap = null;
  var lastToggle = 0;
  var observer = null;

  function ensureDialog() {
    if (dialog) {
      return;
    }

    dialog = document.createElement("dialog");
    dialog.className = "viewer";
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-labelledby", "viewer-captured");
    dialog.setAttribute("aria-describedby", "viewer-caption");

    frame = document.createElement("div");
    frame.className = "viewer-frame";

    var top = document.createElement("div");
    top.className = "viewer-top";
    position = document.createElement("p");
    position.className = "viewer-position";
    position.id = "viewer-position";
    position.setAttribute("aria-live", "polite");
    closeButton = document.createElement("button");
    closeButton.type = "button";
    closeButton.className = "viewer-button viewer-close";
    closeButton.setAttribute("aria-label", "Close viewer");
    closeButton.textContent = "Close";
    top.appendChild(position);
    top.appendChild(closeButton);

    stage = document.createElement("div");
    stage.className = "viewer-stage";
    wrap = document.createElement("div");
    wrap.className = "viewer-media-wrap";
    stage.appendChild(wrap);

    var controls = document.createElement("div");
    controls.className = "viewer-controls";
    prevButton = controlButton("viewer-prev", "Previous item", "Previous");
    nextButton = controlButton("viewer-next", "Next item", "Next");
    zoomOutButton = controlButton("viewer-zoom-out", "Zoom out", "−");
    zoomInButton = controlButton("viewer-zoom-in", "Zoom in", "+");
    resetButton = controlButton("viewer-reset", "Reset zoom", "Reset");
    originalLink = document.createElement("a");
    originalLink.className = "viewer-original";
    originalLink.target = "_blank";
    originalLink.rel = "noopener";
    originalLink.textContent = "Open original";
    controls.appendChild(prevButton);
    controls.appendChild(nextButton);
    controls.appendChild(zoomOutButton);
    controls.appendChild(zoomInButton);
    controls.appendChild(resetButton);
    controls.appendChild(originalLink);

    var caption = document.createElement("div");
    caption.className = "viewer-caption";
    caption.id = "viewer-caption";
    captured = document.createElement("p");
    captured.className = "viewer-captured";
    captured.id = "viewer-captured";
    error = document.createElement("p");
    error.className = "viewer-error";
    error.hidden = true;
    error.textContent =
      "This item cannot be played here. Use the original link to open it.";
    caption.appendChild(captured);
    caption.appendChild(error);

    frame.appendChild(top);
    frame.appendChild(stage);
    frame.appendChild(controls);
    frame.appendChild(caption);
    dialog.appendChild(frame);
    document.body.appendChild(dialog);

    dialog.addEventListener("click", function (event) {
      if (event.target === dialog) {
        close();
      }
    });
    dialog.addEventListener("cancel", function (event) {
      event.preventDefault();
      close();
    });
    dialog.addEventListener("close", function () {
      state.open = false;
    });
    dialog.addEventListener("keydown", onKeyDown);

    closeButton.addEventListener("click", close);
    prevButton.addEventListener("click", function () {
      step(-1);
    });
    nextButton.addEventListener("click", function () {
      step(1);
    });
    zoomInButton.addEventListener("click", function () {
      zoomAt(1.3, { x: 0, y: 0 });
    });
    zoomOutButton.addEventListener("click", function () {
      zoomAt(1 / 1.3, { x: 0, y: 0 });
    });
    resetButton.addEventListener("click", resetZoom);

    stage.addEventListener("pointerdown", onPointerDown);
    stage.addEventListener("pointermove", onPointerMove);
    stage.addEventListener("pointerup", onPointerUp);
    stage.addEventListener("pointercancel", onPointerCancel);
    stage.addEventListener("dblclick", onDoubleClick);
    stage.addEventListener("wheel", onWheel, { passive: false });
  }

  function controlButton(className, label, text) {
    var button = document.createElement("button");
    button.type = "button";
    button.className = "viewer-button " + className;
    button.setAttribute("aria-label", label);
    button.textContent = text;
    return button;
  }

  function onKeyDown(event) {
    if (event.key === "Tab") {
      trapFocus(event);
      return;
    }

    var videoFocused =
      state.mediaKind === "video" && document.activeElement === state.media;

    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      if (videoFocused) {
        return;
      }
      event.preventDefault();
      step(event.key === "ArrowLeft" ? -1 : 1);
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }

    if (state.mediaKind !== "image") {
      return;
    }

    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      zoomAt(1.25, { x: 0, y: 0 });
    } else if (event.key === "-" || event.key === "_") {
      event.preventDefault();
      zoomAt(1 / 1.25, { x: 0, y: 0 });
    } else if (event.key === "0") {
      event.preventDefault();
      resetZoom();
    }
  }

  function trapFocus(event) {
    var focusables = dialog.querySelectorAll(
      "button:not([disabled]), a[href], video[controls], [tabindex]:not([tabindex='-1'])"
    );
    if (focusables.length === 0) {
      return;
    }
    var first = focusables[0];
    var last = focusables[focusables.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function openTile(tile) {
    var items = collectAlbum(document);
    var item = null;
    var index = -1;
    for (var cursor = 0; cursor < items.length; cursor += 1) {
      if (items[cursor].element === tile) {
        item = items[cursor];
        index = cursor;
        break;
      }
    }
    if (!item) {
      return false;
    }
    state.items = items;
    state.opener = tile;
    showAt(index, false);
    return true;
  }

  function openByPath(path) {
    var items = collectAlbum(document);
    var index = albumIndexOf(items, path);
    if (index === -1) {
      return false;
    }
    state.items = items;
    state.opener = null;
    showAt(index, true);
    return true;
  }

  function showAt(index, deepLinked) {
    ensureDialog();
    state.index = index;
    state.open = true;
    state.scale = MIN_ZOOM;
    state.x = 0;
    state.y = 0;

    if (!dialog.open) {
      dialog.showModal();
    }
    showItem();

    if (deepLinked) {
      closeButton.focus();
    } else if (state.opener && state.opener.isConnected) {
      closeButton.focus();
    }

    updateUrl(state.items[index].path);
  }

  function captionText(item) {
    return item.model ? item.captured + " · " + item.model : item.captured;
  }

  function showItem() {
    var item = state.items[state.index];
    resetZoom();
    cancelOriginal();
    teardownVideo();
    error.hidden = true;
    measureStage();

    if (item.kind === "video") {
      renderVideo(item);
    } else {
      renderImage(item);
    }

    position.textContent =
      "Item " + (state.index + 1) + " of " + state.items.length;
    captured.textContent = captionText(item);
    originalLink.href = item.mediaUrl;
    originalLink.setAttribute(
      "aria-label",
      "Open original: " + (item.name || item.path)
    );
    prevButton.disabled = state.index === 0;
    nextButton.disabled = state.index === state.items.length - 1;
    zoomInButton.disabled = item.kind === "video";
    zoomOutButton.disabled = item.kind === "video";
    resetButton.disabled = item.kind === "video";
    if (item.kind !== "image") {
      scheduleOriginal(function () {
        preloadNeighbours();
      });
    }
  }

  function renderImage(item) {
    var image = document.createElement("img");
    image.className = "viewer-image";
    image.alt = item.alt;
    image.decoding = "async";
    image.draggable = false;
    if (item.thumbUrl) {
      image.src = item.thumbUrl;
    } else {
      image.src = item.mediaUrl;
    }
    wrap.replaceChildren(image);
    state.media = image;
    state.mediaKind = "image";

    if (item.thumbUrl && item.width && item.height) {
      image.classList.add("is-placeholder");
      var box = placeholderBox(item.width, item.height, state.stageWidth, state.stageHeight);
      if (box) {
        image.style.width = box.width + "px";
        image.style.height = box.height + "px";
      }
    }

    image.addEventListener("load", measureFit);

    if (item.thumbUrl) {
      scheduleOriginal(function () {
        loadOriginal(item, image);
      });
    } else {
      scheduleOriginal(function () {
        preloadNeighbours();
      });
    }

    if (image.complete) {
      measureFit();
    }
  }

  function scheduleOriginal(callback) {
    state.originalTimer = window.setTimeout(function () {
      state.originalTimer = null;
      callback();
    }, ORIGINAL_DELAY_MS);
  }

  function loadOriginal(item, image) {
    var original = new Image();
    var settle = function () {
      if (state.originalLoader !== original) {
        return;
      }
      state.originalLoader = null;
      preloadNeighbours();
    };
    original.onload = function () {
      if (state.media !== image) {
        return;
      }
      image.classList.remove("is-placeholder");
      image.style.width = "";
      image.style.height = "";
      image.src = item.mediaUrl;
      image.classList.add("is-original");
      settle();
    };
    original.onerror = settle;
    state.originalLoader = original;
    original.src = item.mediaUrl;
  }

  function cancelOriginal() {
    if (state.originalTimer) {
      clearTimeout(state.originalTimer);
      state.originalTimer = null;
    }
    var loader = state.originalLoader;
    state.originalLoader = null;
    if (loader) {
      // Dropping the source lets the browser abort a still-running download.
      loader.onload = null;
      loader.onerror = null;
      loader.removeAttribute("src");
    }
  }

  function renderVideo(item) {
    var video = document.createElement("video");
    video.className = "viewer-video";
    video.controls = true;
    video.playsInline = true;
    video.preload = "metadata";
    video.setAttribute("aria-label", item.name || item.path);
    if (item.thumbUrl) {
      video.poster = item.thumbUrl;
    }
    video.addEventListener("error", function () {
      error.hidden = false;
    });
    video.src = item.mediaUrl;
    wrap.replaceChildren(video);
    state.media = video;
    state.mediaKind = "video";
  }

  function teardownVideo() {
    var media = state.media;
    if (!media || media.tagName !== "VIDEO") {
      return;
    }
    try {
      media.pause();
      media.currentTime = 0;
    } catch (ignored) {
      // The element may already have been detached.
    }
    media.removeAttribute("src");
    while (media.firstChild) {
      media.removeChild(media.firstChild);
    }
    media.load();
  }

  function measureFit() {
    var media = state.media;
    if (!media || state.mediaKind !== "image") {
      return;
    }
    var item = state.items[state.index];
    var stageRect = stage.getBoundingClientRect();
    var naturalWidth = media.naturalWidth || media.width;
    var naturalHeight = media.naturalHeight || media.height;
    if (item && item.width && item.height) {
      naturalWidth = item.width;
      naturalHeight = item.height;
    }
    if (!naturalWidth || !naturalHeight || !stageRect.width || !stageRect.height) {
      return;
    }
    var fit = Math.min(
      stageRect.width / naturalWidth,
      stageRect.height / naturalHeight
    );
    state.stageWidth = stageRect.width;
    state.stageHeight = stageRect.height;
    state.baseWidth = naturalWidth * fit;
    state.baseHeight = naturalHeight * fit;
    clampState();
    applyTransform();
  }

  function measureStage() {
    if (!stage) {
      return;
    }
    var rect = stage.getBoundingClientRect();
    state.stageWidth = rect.width;
    state.stageHeight = rect.height;
  }

  function applyTransform() {
    if (!state.media || state.mediaKind !== "image") {
      return;
    }
    state.media.style.transform =
      "translate(" +
      state.x +
      "px, " +
      state.y +
      "px) scale(" +
      state.scale +
      ")";
  }

  function resetZoom() {
    state.scale = MIN_ZOOM;
    state.x = 0;
    state.y = 0;
    applyTransform();
  }

  function zoomAt(factor, point) {
    if (state.mediaKind !== "image") {
      return;
    }
    var next = zoomAroundPoint(state.scale, state.x, state.y, factor, point);
    state.scale = next.scale;
    state.x = next.x;
    state.y = next.y;
    clampState();
    applyTransform();
  }

  function clampState() {
    var clamped = clampPan(
      state.x,
      state.y,
      state.scale,
      state.baseWidth,
      state.baseHeight,
      state.stageWidth,
      state.stageHeight
    );
    state.x = clamped.x;
    state.y = clamped.y;
  }

  function pointFromEvent(event) {
    var rect = stage.getBoundingClientRect();
    return {
      x: event.clientX - rect.left - rect.width / 2,
      y: event.clientY - rect.top - rect.height / 2
    };
  }

  function onPointerDown(event) {
    if (state.mediaKind !== "image") {
      return;
    }
    if (event.pointerType === "mouse" && event.button !== 0) {
      return;
    }
    try {
      stage.setPointerCapture(event.pointerId);
    } catch (ignored) {
      // Pointer capture is best-effort.
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });

    if (pointers.size === 1) {
      gesture = {
        startX: event.clientX,
        startY: event.clientY,
        startTime: Date.now(),
        x: state.x,
        y: state.y,
        pinch: false,
        moved: false
      };
    } else if (pointers.size === 2) {
      var values = Array.from(pointers.values());
      gesture = {
        pinch: true,
        startDistance: distance(values[0], values[1]),
        startScale: state.scale,
        startMid: midpoint(values[0], values[1]),
        x: state.x,
        y: state.y,
        moved: true
      };
    }
  }

  function onPointerMove(event) {
    if (!pointers.has(event.pointerId) || state.mediaKind !== "image") {
      return;
    }
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });

    if (pointers.size === 1 && gesture && !gesture.pinch) {
      var dx = event.clientX - gesture.startX;
      var dy = event.clientY - gesture.startY;
      if (Math.abs(dx) > 4 || Math.abs(dy) > 4) {
        gesture.moved = true;
      }
      if (state.scale > MIN_ZOOM) {
        state.x = gesture.x + dx;
        state.y = gesture.y + dy;
        clampState();
        applyTransform();
      }
    } else if (pointers.size === 2 && gesture && gesture.pinch) {
      var values = Array.from(pointers.values());
      var currentDistance = distance(values[0], values[1]);
      if (gesture.startDistance <= 0) {
        return;
      }
      var nextScale = clampZoom(
        gesture.startScale * (currentDistance / gesture.startDistance)
      );
      var ratio = nextScale / gesture.startScale;
      var currentMid = midpoint(values[0], values[1]);
      state.scale = nextScale;
      state.x = currentMid.x - (gesture.startMid.x - gesture.x) * ratio;
      state.y = currentMid.y - (gesture.startMid.y - gesture.y) * ratio;
      clampState();
      applyTransform();
    }
  }

  function onPointerUp(event) {
    if (!pointers.has(event.pointerId)) {
      return;
    }
    var wasPinch = gesture && gesture.pinch;
    pointers.delete(event.pointerId);

    if (pointers.size === 0 && gesture) {
      var dx = event.clientX - gesture.startX;
      var dy = event.clientY - gesture.startY;
      var duration = Date.now() - gesture.startTime;

      if (!wasPinch && state.scale === MIN_ZOOM && isSwipe(dx, dy, state.stageWidth)) {
        step(dx < 0 ? 1 : -1);
      } else if (!wasPinch && !gesture.moved && duration < 350) {
        if (event.pointerType !== "mouse") {
          handleTap(event);
        }
      }
      gesture = null;
    } else if (wasPinch && pointers.size === 1) {
      var remaining = Array.from(pointers.values())[0];
      gesture = {
        startX: remaining.x,
        startY: remaining.y,
        startTime: Date.now(),
        x: state.x,
        y: state.y,
        pinch: false,
        moved: true
      };
    } else if (!wasPinch) {
      gesture = {
        startX: event.clientX,
        startY: event.clientY,
        startTime: Date.now(),
        x: state.x,
        y: state.y,
        pinch: false,
        moved: false
      };
    }
  }

  function onPointerCancel(event) {
    pointers.delete(event.pointerId);
    if (pointers.size === 0) {
      gesture = null;
    }
  }

  function handleTap(event) {
    var now = Date.now();
    var point = pointFromEvent(event);
    if (
      lastTap &&
      now - lastTap.time < 300 &&
      Math.abs(event.clientX - lastTap.x) < 30 &&
      Math.abs(event.clientY - lastTap.y) < 30
    ) {
      lastTap = null;
      toggleZoom(point);
    } else {
      lastTap = { time: now, x: event.clientX, y: event.clientY };
    }
  }

  function onDoubleClick(event) {
    if (state.mediaKind !== "image") {
      return;
    }
    toggleZoom(pointFromEvent(event));
  }

  function toggleZoom(point) {
    var now = Date.now();
    if (now - lastToggle < 350) {
      return;
    }
    lastToggle = now;
    if (state.scale > MIN_ZOOM) {
      resetZoom();
    } else {
      zoomAt(DOUBLE_TAP_ZOOM, point);
    }
  }

  function onWheel(event) {
    if (state.mediaKind !== "image") {
      return;
    }
    event.preventDefault();
    var factor = Math.exp(-event.deltaY * 0.0015);
    zoomAt(factor, pointFromEvent(event));
  }

  function distance(left, right) {
    return Math.hypot(left.x - right.x, left.y - right.y);
  }

  function midpoint(left, right) {
    var rect = stage.getBoundingClientRect();
    return {
      x: (left.x + right.x) / 2 - rect.left - rect.width / 2,
      y: (left.y + right.y) / 2 - rect.top - rect.height / 2
    };
  }

  function step(direction) {
    var next = nextIndex(state.index, direction, state.items.length);
    if (next === -1) {
      return;
    }
    state.index = next;
    showItem();
    updateUrl(state.items[next].path);
  }

  function close() {
    if (!state.open || !dialog) {
      return;
    }
    var opener = state.opener;
    var lastItem = state.items[state.index];
    cancelOriginal();
    teardownVideo();
    state.open = false;
    state.opener = null;
    state.items = [];
    state.index = -1;
    if (dialog.open) {
      dialog.close();
    }
    updateUrl(null);
    if (opener && opener.isConnected) {
      opener.focus();
    } else if (lastItem && lastItem.element && lastItem.element.isConnected) {
      lastItem.element.focus();
    } else {
      focusFallback();
    }
  }

  function focusFallback() {
    var target = document.querySelector("main h1") || document.querySelector("main");
    if (!target) {
      return;
    }
    if (!target.hasAttribute("tabindex")) {
      target.setAttribute("tabindex", "-1");
    }
    target.focus();
  }

  function preloadNeighbours() {
    var connection = navigator.connection || {};
    if (connection.saveData || !state.open) {
      return;
    }
    preloadNeighbour(state.index - 1);
    preloadNeighbour(state.index + 1);
  }

  function preloadNeighbour(index) {
    var item = state.items[index];
    if (!item || item.kind !== "image" || item === state.items[state.index]) {
      return;
    }
    var image = new Image();
    image.src = item.mediaUrl;
  }

  function updateUrl(path) {
    var next = buildItemUrl(window.location.href, path);
    window.history.replaceState(null, "", next);
  }

  function refreshAlbum() {
    if (!state.open) {
      return;
    }
    var current = state.items[state.index];
    var items = collectAlbum(document);
    var index = current ? albumIndexOf(items, current.path) : -1;
    if (index === -1) {
      close();
      return;
    }
    state.items = items;
    state.index = index;
    state.opener = items[index].element;
    position.textContent =
      "Item " + (index + 1) + " of " + items.length;
    prevButton.disabled = index === 0;
    nextButton.disabled = index === items.length - 1;
  }

  function tryPendingItem() {
    if (!state.pendingItem || state.open) {
      return;
    }
    if (collectAlbum(document).length === 0) {
      return;
    }
    if (openByPath(state.pendingItem)) {
      state.pendingItem = null;
      stopObserver();
    }
  }

  function stopObserver() {
    if (observer) {
      observer.disconnect();
      observer = null;
    }
  }

  function watchForAlbum() {
    if (observer || typeof MutationObserver === "undefined") {
      return;
    }
    observer = new MutationObserver(function () {
      tryPendingItem();
    });
    observer.observe(document.body || document.documentElement, {
      childList: true,
      subtree: true
    });
    window.setTimeout(stopObserver, 10000);
  }

  function onDocumentClick(event) {
    if (event.defaultPrevented || event.button !== 0) {
      return;
    }
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
      return;
    }
    var target = event.target;
    var tile = target && target.closest ? target.closest("a.tile-link") : null;
    if (!tile) {
      return;
    }
    try {
      if (openTile(tile)) {
        event.preventDefault();
      }
    } catch (error) {
      if (typeof console !== "undefined" && console.error) {
        console.error("media viewer could not open", error);
      }
    }
  }

  function supportsDialog() {
    return (
      typeof HTMLDialogElement !== "undefined" &&
      typeof document.createElement("dialog").showModal === "function"
    );
  }

  function init() {
    if (!supportsDialog()) {
      return;
    }
    state.pendingItem = parseItemParam(window.location.search);
    document.addEventListener("click", onDocumentClick);
    document.addEventListener(RESULTS_EVENT, function () {
      refreshAlbum();
      tryPendingItem();
    });
    window.addEventListener("resize", function () {
      if (!state.open || state.mediaKind !== "image") {
        return;
      }
      measureStage();
      measureFit();
      clampState();
      applyTransform();
    });

    var ready = function () {
      if (state.pendingItem) {
        tryPendingItem();
        if (state.pendingItem) {
          watchForAlbum();
        }
      }
    };

    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", ready);
    } else {
      ready();
    }
  }

  globalThis.__mediaCatalogViewer = {
    MIN_ZOOM: MIN_ZOOM,
    MAX_ZOOM: MAX_ZOOM,
    DOUBLE_TAP_ZOOM: DOUBLE_TAP_ZOOM,
    ORIGINAL_DELAY_MS: ORIGINAL_DELAY_MS,
    clamp: clamp,
    clampZoom: clampZoom,
    clampPan: clampPan,
    zoomAroundPoint: zoomAroundPoint,
    parseItemParam: parseItemParam,
    buildItemUrl: buildItemUrl,
    albumIndexOf: albumIndexOf,
    nextIndex: nextIndex,
    swipeThreshold: swipeThreshold,
    isSwipe: isSwipe,
    tileFromElement: tileFromElement,
    captionText: captionText,
    fittedBox: fittedBox,
    placeholderBox: placeholderBox
  };

  if (typeof document !== "undefined" && typeof window !== "undefined") {
    init();
  }
})();
