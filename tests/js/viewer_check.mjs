import fs from "node:fs";
import vm from "node:vm";

const [, , assetPath] = process.argv;
const code = fs.readFileSync(assetPath, "utf8");
const context = { console, URL, URLSearchParams };
vm.createContext(context);
vm.runInContext(code, context);

const api = context.__mediaCatalogViewer;
if (!api) {
  throw new Error("viewer.js did not expose __mediaCatalogViewer");
}

const album = [{ path: "2024/05/a.jpg" }, { path: "2024/05/b.jpg" }];
const slideshowAlbum = [
  { path: "2024/05/a.jpg", kind: "image" },
  { path: "fixture/clip.mp4", kind: "video" },
  { path: "2024/05/b.jpg", kind: "image" },
  { path: "fixture/clip2.mp4", kind: "video" }
];
const slideshowVideoTail = [
  { path: "2024/05/a.jpg", kind: "image" },
  { path: "2024/05/b.jpg", kind: "image" },
  { path: "fixture/clip.mp4", kind: "video" }
];
const slideshowSingleImageAlbum = [
  { path: "2024/05/a.jpg", kind: "image" },
  { path: "fixture/clip.mp4", kind: "video" }
];
const slideshowNoImageAlbum = [
  { path: "fixture/clip.mp4", kind: "video" },
  { path: "fixture/clip2.mp4", kind: "video" }
];

const fakeTile = {
  getAttribute(name) {
    return {
      href: "/media/2024/05/a.jpg",
      "data-item-path": "2024/05/a.jpg",
      "data-kind": "image",
      "data-captured": "2024-05-01 10:00:00",
      "data-thumb-url": "/thumbs/2024/05/a.jpg.webp?v=1",
      "data-width": "4000",
      "data-height": "3000",
      "data-f-number": "1.8",
      "data-exposure": "1/125",
      "data-iso": "100",
      "data-focal": "26",
      "data-lat": "52.520008",
      "data-lon": "13.404954"
    }[name] || null;
  },
  querySelector(selector) {
    return selector === ".model" ? { textContent: " iPhone 15 Pro " } : null;
  }
};
const tile = api.tileFromElement(fakeTile);
const bareTile = api.tileFromElement({
  getAttribute: fakeTile.getAttribute,
  querySelector() {
    return null;
  }
});
const noMetadataTile = api.tileFromElement({
  getAttribute() {
    return null;
  },
  querySelector() {
    return null;
  }
});
function withSize(base, size) {
  return {
    getAttribute(name) {
      return name === "data-size" ? size : base.getAttribute(name);
    },
    querySelector: base.querySelector
  };
}
function withAttributes(base, attributes) {
  return {
    getAttribute(name) {
      return Object.prototype.hasOwnProperty.call(attributes, name)
        ? attributes[name]
        : base.getAttribute(name);
    },
    querySelector: base.querySelector
  };
}
const sizedTile = api.tileFromElement(withSize(fakeTile, "3400000"));
const sizedBareTile = api.tileFromElement(withSize(bareTile.element, "532000"));
const badSizeTile = api.tileFromElement(withSize(fakeTile, "12abc"));
const videoTile = api.tileFromElement(
  withAttributes(fakeTile, {
    href: "/media/fixture/clip.mp4",
    "data-item-path": "fixture/clip.mp4",
    "data-kind": "video",
    "data-captured": "2024-05-03 09:00:00",
    "data-width": "1920",
    "data-height": "1080",
    "data-size": "1500000"
  })
);
const roundedTile = api.tileFromElement(
  withAttributes(fakeTile, {
    "data-width": "4032",
    "data-height": "3024"
  })
);
const missingDimensionTile = api.tileFromElement(
  withAttributes(fakeTile, {
    "data-height": null,
    "data-size": "3400000"
  })
);

const result = {
  item: api.parseItemParam("?q=alpha&item=2024/05/a.jpg"),
  noItem: api.parseItemParam("?q=alpha"),
  encodedItem: api.buildItemUrl("/search.html?q=alpha&kind=video", "2024/05/a.jpg"),
  preservedItem: api.buildItemUrl("/search.html?q=alpha&item=2024%2F05%2Fa.jpg&kind=video", null),
  fragment: api.buildItemUrl("/events/trip/?item=x#lisbon", null),
  found: api.albumIndexOf(album, "2024/05/b.jpg"),
  missing: api.albumIndexOf(album, "2024/05/c.jpg"),
  next: api.nextIndex(0, 1, 2),
  previous: api.nextIndex(1, -1, 2),
  beforeFirst: api.nextIndex(0, -1, 2),
  afterLast: api.nextIndex(1, 1, 2),
  slideshowStartOnImage: api.slideshowTarget(slideshowAlbum, 0, true),
  slideshowStartOnMiddleImage: api.slideshowTarget(slideshowAlbum, 2, true),
  slideshowAdvanceSkipsVideos: api.slideshowTarget(slideshowAlbum, 0, false),
  slideshowAdvanceStops: api.slideshowTarget(slideshowAlbum, 2, false),
  slideshowStartOnVideo: api.slideshowTarget(slideshowAlbum, 1, true),
  slideshowStartOnLastItem: api.slideshowTarget(slideshowAlbum, 3, true),
  slideshowStartOnVideoWithoutImageAfter: api.slideshowTarget(
    slideshowVideoTail,
    2,
    true
  ),
  slideshowAdvanceFromLastImage: api.slideshowTarget(slideshowVideoTail, 1, false),
  slideshowSingleImage: api.slideshowTarget(slideshowSingleImageAlbum, 0, true),
  slideshowNoImages: api.slideshowTarget(slideshowNoImageAlbum, 0, true),
  slideshowFirstImage: api.firstImageIndex(slideshowAlbum, 0),
  slideshowFirstImageAfterVideo: api.firstImageIndex(slideshowAlbum, 1),
  slideshowFirstImageMissing: api.firstImageIndex(slideshowNoImageAlbum, 0),
  slideshowImageCount: api.imageItemCount(slideshowAlbum),
  slideshowInterval: api.SLIDESHOW_INTERVAL_MS,
  idleChrome: api.IDLE_CHROME_MS,
  zoomLow: api.clampZoom(0.5),
  zoomHigh: api.clampZoom(9),
  panFit: api.clampPan(100, 100, 1, 200, 100, 400, 300),
  panBounds: api.clampPan(400, 400, 2, 300, 200, 400, 300),
  zoomPoint: api.zoomAroundPoint(1, 0, 0, 2, { x: 50, y: 20 }),
  zoomReset: api.zoomAroundPoint(2, -30, -10, 0.25, { x: 10, y: 10 }),
  thresholdWide: api.swipeThreshold(1000),
  thresholdNarrow: api.swipeThreshold(200),
  swipeYes: api.isSwipe(150, 10, 1000),
  swipeNo: api.isSwipe(100, 200, 1000),
  swipeSmall: api.isSwipe(30, 0, 200),
  tileName: tile.name,
  tileCaptured: tile.captured,
  tileModel: tile.model,
  tileCaption: api.captionText(tile),
  bareTileModel: bareTile.model,
  bareTileCaption: api.captionText(bareTile),
  sizedTileSize: sizedTile.size,
  sizedTileCaption: api.captionText(sizedTile),
  sizedBareTileCaption: api.captionText(sizedBareTile),
  badSizeTileSize: badSizeTile.size,
  badSizeTileCaption: api.captionText(badSizeTile),
  videoCaption: api.captionText(videoTile),
  roundedCaption: api.captionText(roundedTile),
  missingDimensionCaption: api.captionText(missingDimensionTile),
  megapixelsLandscape: api.formatMegapixels(4000, 3000),
  megapixelsVideo: api.formatMegapixels(1920, 1080),
  megapixelsRounded: api.formatMegapixels(4032, 3024),
  megapixelsSmall: api.formatMegapixels(640, 480),
  megapixelsMissing: api.formatMegapixels(4000, null),
  sizeBytes: api.formatFileSize(845),
  sizeKilobytes: api.formatFileSize(532000),
  sizeRoundsUpToMegabytes: api.formatFileSize(999950),
  sizeMegabytes: api.formatFileSize(3400000),
  sizeGigabytes: api.formatFileSize(1240000000),
  sizeUnknown: api.formatFileSize(null),
  tileFNumber: tile.fNumber,
  tileExposure: tile.exposure,
  tileIso: tile.iso,
  tileFocal: tile.focal,
  tileLatitude: tile.latitude,
  tileLongitude: tile.longitude,
  metadataFull: api.metadataText(tile),
  metadataPartial: api.metadataText({ iso: 100 }),
  metadataEmpty: api.metadataText({}),
  exposureFraction: api.formatExposure("1/125"),
  exposureSeconds: api.formatExposure("4/1"),
  exposureDecimal: api.formatExposure("3/2"),
  exposureInvalid: api.formatExposure("not-a-number"),
  coordinates: api.formatCoordinates(52.520008, 13.404954),
  locationUrl: api.locationUrl(52.520008, 13.404954),
  locationLabel: api.locationLabel(52.520008, 13.404954),
  previewBbox: api.previewBbox(52.520008, 13.404954),
  previewUrl: api.previewUrl(52.520008, 13.404954),
  previewTitle: api.previewTitle(52.520008, 13.404954),
  previewDelay: api.PREVIEW_DELAY_MS,
  hasLocation: api.hasLocation(tile),
  noLocation: api.hasLocation(noMetadataTile),
  tileAlt: tile.alt,
  tileWidth: tile.width,
  tileHeight: tile.height,
  fitWide: api.fittedBox(4000, 3000, 1000, 800),
  fitTall: api.fittedBox(3000, 4000, 1000, 800),
  fitMissing: api.fittedBox(0, 0, 1000, 800),
  placeholderWide: api.placeholderBox(4000, 3000, 1000, 800),
  placeholderTall: api.placeholderBox(3000, 4000, 1000, 800),
  placeholderMissing: api.placeholderBox(0, 0, 1000, 800),
  originalDelay: api.ORIGINAL_DELAY_MS
};

console.log(JSON.stringify(result));
