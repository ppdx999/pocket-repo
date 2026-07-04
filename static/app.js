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
