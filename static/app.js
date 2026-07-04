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

  recordRecent();
  renderRecent();

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
