(function () {
  "use strict";

  function normalize(value) {
    return String(value == null ? "" : value).toLocaleLowerCase();
  }

  function filterItems(items, filters) {
    var text = normalize(filters.text);
    var kind = filters.kind ? String(filters.kind) : "";
    var year = filters.year ? String(filters.year) : "";
    var month = filters.month ? String(filters.month).padStart(2, "0") : "";

    return items.filter(function (item) {
      if (text && normalize(item.path).indexOf(text) === -1) {
        return false;
      }
      if (kind && item.kind !== kind) {
        return false;
      }
      if (year && String(item.year) !== year) {
        return false;
      }
      if (month && String(item.month).padStart(2, "0") !== month) {
        return false;
      }
      return true;
    });
  }

  function loadCatalog() {
    return fetch("/catalog/index.json", { cache: "no-cache" })
      .then(function (response) {
        if (!response.ok) {
          throw new Error("could not load catalog index");
        }
        return response.json();
      })
      .then(function (index) {
        var work = index.years.map(function (year) {
          return fetch("/" + year.shard, { cache: "no-cache" })
            .then(function (response) {
              if (!response.ok) {
                throw new Error("could not load shard " + year.shard);
              }
              return response.json();
            })
            .then(function (shard) {
              return shard.items.map(function (item) {
                return Object.assign({ year: year.year }, item);
              });
            });
        });
        return Promise.all(work).then(function (groups) {
          return { index: index, items: groups.reduce(function (all, group) {
            return all.concat(group);
          }, []) };
        });
      });
  }

  function formatDuration(ms) {
    if (!ms && ms !== 0) {
      return "";
    }
    var total = Math.round(ms / 1000);
    var hours = Math.floor(total / 3600);
    var minutes = Math.floor((total % 3600) / 60);
    var seconds = total % 60;
    if (hours > 0) {
      return hours + ":" + String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0");
    }
    return minutes + ":" + String(seconds).padStart(2, "0");
  }

  function formatCaptured(value) {
    var text = String(value == null ? "" : value);
    var match = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})/.exec(text);
    if (!match) {
      return text;
    }
    return match[1].slice(2) + "-" + match[2] + "-" + match[3] + " " + match[4] + ":" + match[5];
  }

  function captionText(item) {
    var short = formatCaptured(item.captured_at || "");
    var model = item.model ? String(item.model) : "";
    return model ? short + " " + model : short;
  }

  function buildTile(item) {
    var figure = document.createElement("figure");
    figure.className = "tile";

    var link = document.createElement("a");
    link.className = "tile-link";
    link.href = item.media_url;
    link.dataset.itemPath = item.path;
    link.dataset.kind = item.kind;
    link.dataset.captured = item.captured_at || "";
    if (typeof item.size_bytes === "number") {
      link.dataset.size = String(item.size_bytes);
    }
    if (item.thumb_url) {
      link.dataset.thumbUrl = item.thumb_url;
    }
    if (typeof item.width === "number") {
      link.dataset.width = String(item.width);
    }
    if (typeof item.height === "number") {
      link.dataset.height = String(item.height);
    }

    var name = item.path.split("/").pop();
    var alt = name || ("Media item captured " + (item.captured_at || ""));

    if (item.thumb_available && item.thumb_url) {
      var image = document.createElement("img");
      image.src = item.thumb_url;
      image.alt = alt;
      image.loading = "lazy";
      image.decoding = "async";
      link.appendChild(image);
    } else {
      var placeholder = document.createElement("span");
      placeholder.className = "placeholder";
      placeholder.setAttribute("role", "img");
      placeholder.setAttribute("aria-label", alt);
      placeholder.textContent = "No preview";
      link.appendChild(placeholder);
    }

    if (item.kind === "video") {
      var badge = document.createElement("span");
      badge.className = "badge video";
      var duration = formatDuration(item.duration_ms);
      badge.textContent = duration ? "Video · " + duration : "Video";
      link.appendChild(badge);
    }

    var caption = document.createElement("figcaption");
    var time = document.createElement("time");
    time.textContent = formatCaptured(item.captured_at || "");
    caption.appendChild(time);
    if (item.model) {
      var modelSpan = document.createElement("span");
      modelSpan.className = "model";
      modelSpan.textContent = item.model;
      caption.appendChild(modelSpan);
    }
    link.appendChild(caption);
    figure.appendChild(link);
    return figure;
  }

  function filtersFromParams(search) {
    var params = new URLSearchParams(search || "");
    return {
      text: params.get("q") || "",
      kind: params.get("kind") || "",
      year: params.get("year") || "",
      month: params.get("month") || ""
    };
  }

  function filtersToQuery(filters) {
    var params = new URLSearchParams();
    [["q", filters.text], ["kind", filters.kind], ["year", filters.year], ["month", filters.month]]
      .forEach(function (pair) {
        if (pair[1]) {
          params.set(pair[0], pair[1]);
        }
      });
    return params.toString();
  }

  function currentFilters() {
    return {
      text: document.getElementById("q").value,
      kind: document.getElementById("kind").value,
      year: document.getElementById("year").value,
      month: document.getElementById("month").value
    };
  }

  function applyFilters(items) {
    var filters = currentFilters();
    var matches = filterItems(items, filters);
    var results = document.getElementById("results");
    results.replaceChildren();
    matches.forEach(function (item) {
      results.appendChild(buildTile(item));
    });
    document.getElementById("status").textContent =
      matches.length + " of " + items.length + " items";

    var params = new URLSearchParams(filtersToQuery(filters));
    var item = new URLSearchParams(window.location.search).get("item");
    if (item) {
      params.set("item", item);
    }
    var query = params.toString();
    var next = window.location.pathname + (query ? "?" + query : "");
    window.history.replaceState(null, "", next);

    document.dispatchEvent(
      new CustomEvent("media-catalog:results-rendered", {
        detail: { count: matches.length }
      })
    );
    return matches;
  }

  function populateSelects(index) {
    var yearSelect = document.getElementById("year");
    var monthSelect = document.getElementById("month");
    var months = {};
    index.years.forEach(function (year) {
      var option = document.createElement("option");
      option.value = String(year.year);
      option.textContent = String(year.year);
      yearSelect.appendChild(option);
      (year.months || []).forEach(function (month) {
        months[month.month] = true;
      });
    });
    Object.keys(months)
      .map(Number)
      .sort(function (a, b) { return b - a; })
      .forEach(function (month) {
        var option = document.createElement("option");
        option.value = String(month).padStart(2, "0");
        option.textContent = String(month).padStart(2, "0");
        monthSelect.appendChild(option);
      });
  }

  function restoreFiltersFromUrl() {
    var filters = filtersFromParams(window.location.search);
    document.getElementById("q").value = filters.text;
    document.getElementById("kind").value = filters.kind;
    document.getElementById("year").value = filters.year;
    document.getElementById("month").value = filters.month;
  }

  function init() {
    var state = { items: [], index: null };
    loadCatalog()
      .then(function (result) {
        state.items = result.items;
        state.index = result.index;
        populateSelects(result.index);
        restoreFiltersFromUrl();
        applyFilters(state.items);
        ["q", "kind", "year", "month"].forEach(function (id) {
          var element = document.getElementById(id);
          element.addEventListener("input", function () { applyFilters(state.items); });
          element.addEventListener("change", function () { applyFilters(state.items); });
        });
      })
      .catch(function (error) {
        document.getElementById("status").textContent =
          "The catalog is not available yet. Browse the catalog by year instead. (" +
          error.message + ")";
      });
  }

  if (typeof document !== "undefined" && typeof fetch === "function") {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", init);
    } else {
      init();
    }
  }

  globalThis.__mediaCatalogSearch = {
    filterItems: filterItems,
    formatDuration: formatDuration,
    formatCaptured: formatCaptured,
    captionText: captionText,
    filtersFromParams: filtersFromParams,
    filtersToQuery: filtersToQuery
  };
})();
