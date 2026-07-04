// App-specific client behavior (separate from maudliver's runtime.js).
(function () {
  // Copy text to the clipboard. Prefers the async Clipboard API, but falls back
  // to execCommand for non-secure contexts (PocketRepo is served over plain
  // HTTP on a Tailscale IP, where navigator.clipboard is unavailable).
  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(text);
    }
    return new Promise(function (resolve, reject) {
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.top = "-1000px";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      var ok = false;
      try {
        ok = document.execCommand("copy");
      } catch (e) {
        ok = false;
      }
      document.body.removeChild(ta);
      ok ? resolve() : reject(new Error("copy failed"));
    });
  }

  // --- Recently viewed files (client-side, per-device) ---
  var RECENT_KEY = "pocketrepo:recent";
  var RECENT_MAX = 50;

  function readRecent() {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY)) || [];
    } catch (e) {
      return [];
    }
  }

  // On a file view (marked with data-recent-* on the root), record the visit.
  function recordRecent() {
    var root = document.getElementById("maudliver-root");
    if (!root) return;
    var repo = root.getAttribute("data-recent-repo");
    var path = root.getAttribute("data-recent-path");
    if (!repo || !path) return;

    var list = readRecent().filter(function (it) {
      return !(it.repo === repo && it.path === path);
    });
    list.unshift({ repo: repo, path: path, ts: Date.now() });
    if (list.length > RECENT_MAX) list = list.slice(0, RECENT_MAX);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(list));
    } catch (e) {}
  }

  // On the recent-files page, populate the list from localStorage.
  function renderRecent() {
    var listEl = document.getElementById("recent-list");
    if (!listEl) return;
    var list = readRecent();
    if (!list.length) {
      var emptyEl = document.getElementById("recent-empty");
      if (emptyEl) emptyEl.hidden = false;
      return;
    }
    var frag = document.createDocumentFragment();
    list.forEach(function (it) {
      var li = document.createElement("li");
      li.className = "result";

      var a = document.createElement("a");
      a.href = "/repo/" + it.repo + "/blob/" + it.path;

      var dir = it.path.replace(/[^/]*$/, "");
      var base = it.path.slice(dir.length);
      if (dir) {
        var dirSpan = document.createElement("span");
        dirSpan.className = "recent-repo";
        dirSpan.textContent = it.repo + "/" + dir;
        a.appendChild(dirSpan);
      } else {
        var repoSpan = document.createElement("span");
        repoSpan.className = "recent-repo";
        repoSpan.textContent = it.repo + "/";
        a.appendChild(repoSpan);
      }
      var baseSpan = document.createElement("span");
      baseSpan.className = "path-base";
      baseSpan.textContent = base;
      a.appendChild(baseSpan);

      li.appendChild(a);
      frag.appendChild(li);
    });
    listEl.appendChild(frag);
  }

  // --- Fuzzy filter for the repository list (client-side, instant) ---

  // Subsequence fuzzy score: -1 if `needle` isn't a subsequence of `haystack`,
  // otherwise a score that rewards consecutive matches and word boundaries.
  function fuzzyScore(needle, haystack) {
    needle = needle.toLowerCase();
    haystack = haystack.toLowerCase();
    var n = 0;
    var score = 0;
    var lastIdx = -1;
    var run = 0;
    for (var h = 0; h < haystack.length && n < needle.length; h++) {
      if (haystack[h] === needle[n]) {
        var bonus = 1;
        if (lastIdx === h - 1) {
          run += 1;
          bonus += run * 2;
        } else {
          run = 0;
        }
        if (h === 0 || /[^a-z0-9]/.test(haystack[h - 1])) bonus += 3;
        score += bonus;
        lastIdx = h;
        n += 1;
      }
    }
    return n === needle.length ? score : -1;
  }

  function initRepoFilter() {
    var input = document.getElementById("repo-filter");
    var list = document.getElementById("repo-list");
    if (!input || !list) return;
    var empty = document.getElementById("repo-filter-empty");
    var items = Array.prototype.slice.call(list.children); // original order

    function apply() {
      var q = input.value.trim();
      if (!q) {
        items.forEach(function (li) {
          li.style.display = "";
          list.appendChild(li);
        });
        if (empty) empty.hidden = true;
        return;
      }
      var scored = [];
      items.forEach(function (li) {
        var name = li.getAttribute("data-name") || li.textContent;
        var s = fuzzyScore(q, name);
        if (s >= 0) {
          scored.push({ li: li, s: s });
        } else {
          li.style.display = "none";
        }
      });
      scored.sort(function (a, b) {
        return b.s - a.s;
      });
      scored.forEach(function (it) {
        it.li.style.display = "";
        list.appendChild(it.li); // reorder by score
      });
      if (empty) empty.hidden = scored.length > 0;
    }

    input.addEventListener("input", apply);
  }

  // --- Persist tree expansion state per repo/root (localStorage) ---
  function treeKey(model) {
    return (
      "pocketrepo:tree:" + model.repo + ":" + (model.path || "") + ":" + (model.ref || "")
    );
  }

  function isTreeModel(model) {
    return model && Array.isArray(model.expanded) && typeof model.repo === "string";
  }

  function initTreeState() {
    if (!window.maudliver) return;

    // Save on every model update (toggles, restore).
    window.maudliver.onModel = function (model) {
      if (!isTreeModel(model)) return;
      try {
        localStorage.setItem(treeKey(model), JSON.stringify(model.expanded));
      } catch (e) {}
    };

    // Restore saved expansion once on load (one round-trip re-renders expanded).
    var model = window.maudliver.model;
    if (!isTreeModel(model)) return;
    var saved = [];
    try {
      saved = JSON.parse(localStorage.getItem(treeKey(model))) || [];
    } catch (e) {}
    if (saved.length && JSON.stringify(saved) !== JSON.stringify(model.expanded)) {
      window.maudliver.send("RestoreExpanded", { paths: saved });
    }
  }

  recordRecent();
  renderRecent();
  initRepoFilter();
  initTreeState();

  // Delegated so it keeps working after maudliver replaces DOM nodes.
  document.addEventListener("click", function (e) {
    var btn = e.target.closest("[data-copy]");
    if (!btn) return;
    e.preventDefault();
    e.stopPropagation();
    copyText(btn.getAttribute("data-copy")).then(
      function () {
        btn.classList.add("copied");
        setTimeout(function () {
          btn.classList.remove("copied");
        }, 1200);
      },
      function () {
        btn.classList.add("copy-failed");
        setTimeout(function () {
          btn.classList.remove("copy-failed");
        }, 1200);
      }
    );
  });
})();
