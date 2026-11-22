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

const fakeTile = {
  getAttribute(name) {
    return {
      href: "/media/2024/05/a.jpg",
      "data-item-path": "2024/05/a.jpg",
      "data-kind": "image",
      "data-captured": "2024-05-01 10:00:00",
      "data-thumb-url": "/thumbs/2024/05/a.jpg.webp?v=1",
      "data-width": "4000",
      "data-height": "3000"
    }[name] || null;
  },
  querySelector() {
    return null;
  }
};
const tile = api.tileFromElement(fakeTile);

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
  tileAlt: tile.alt,
  tileWidth: tile.width,
  tileHeight: tile.height,
  fitWide: api.fittedBox(4000, 3000, 1000, 800),
  fitTall: api.fittedBox(3000, 4000, 1000, 800),
  fitMissing: api.fittedBox(0, 0, 1000, 800)
};

console.log(JSON.stringify(result));
