import fs from "node:fs";
import vm from "node:vm";

const [, , assetPath, publicRoot] = process.argv;
const code = fs.readFileSync(assetPath, "utf8");
const context = { console, URLSearchParams };
vm.createContext(context);
vm.runInContext(code, context);

const api = context.__mediaCatalogSearch;
if (!api) {
  throw new Error("search.js did not expose __mediaCatalogSearch");
}

const index = JSON.parse(fs.readFileSync(publicRoot + "/catalog/index.json", "utf8"));
const items = [];
for (const year of index.years) {
  const shard = JSON.parse(fs.readFileSync(publicRoot + "/" + year.shard, "utf8"));
  for (const item of shard.items) {
    items.push(Object.assign({ year: year.year }, item));
  }
}

function count(filters) {
  return api.filterItems(items, filters).length;
}

const restored = api.filtersFromParams("?q=alpha&kind=video&year=2024&month=05");
const query = api.filtersToQuery({
  text: "gamma",
  kind: "video",
  year: "2023",
  month: "12"
});

const result = {
  total: items.length,
  text: count({ text: "ALPHA" }),
  video: count({ kind: "video" }),
  year2024: count({ year: 2024 }),
  month05: count({ month: "05" }),
  combined: count({ text: "gamma", kind: "video", year: 2023, month: "12" }),
  noMatch: count({ text: "nothing-matches" }),
  restoredText: restored.text,
  restoredKind: restored.kind,
  restoredYear: restored.year,
  restoredMonth: restored.month,
  captionWithModel: api.captionText({
    captured_at: "2024-05-01 10:00:00",
    model: "iPhone 15 Pro"
  }),
  captionWithoutModel: api.captionText({ captured_at: "2024-05-01 10:00:00" }),
  captionEmptyModel: api.captionText({
    captured_at: "2024-05-01 10:00:00",
    model: ""
  }),
  query: query
};

console.log(JSON.stringify(result));
