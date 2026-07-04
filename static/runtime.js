(function () {
  const endpoint = window.location.pathname;
  const scriptTag = document.querySelector("script[data-model]");
  let model = JSON.parse(scriptTag.getAttribute("data-model"));

  function sendEvent(eventName, params) {
    fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        event: eventName,
        model: model,
        params: params || {},
      }),
    })
      .then(function (res) {
        return res.json();
      })
      .then(function (data) {
        if (data.redirect) {
          window.location.href = data.redirect;
          return;
        }

        // Apply patches
        data.patches.forEach(function (patch) {
          var el = document.getElementById(patch.id);
          if (el) {
            el.outerHTML = patch.html;
          }
        });

        // Update model
        model = data.model;

        // Re-attach event listeners after DOM replacement
        attachListeners();
      });
  }

  function attachListeners() {
    // Click events on elements with data-event (excluding form children)
    document.querySelectorAll("[data-event]").forEach(function (el) {
      if (el.tagName === "FORM") return;
      if (el.closest("form[data-event]")) return;

      el.addEventListener("click", function (e) {
        e.preventDefault();
        var params = {};
        Array.from(el.attributes).forEach(function (attr) {
          if (attr.name.startsWith("data-param-")) {
            params[attr.name.slice("data-param-".length)] = attr.value;
          }
        });
        sendEvent(el.getAttribute("data-event"), params);
      });
    });

    // Form submit events
    document.querySelectorAll("form[data-event]").forEach(function (form) {
      form.addEventListener("submit", function (e) {
        e.preventDefault();
        const formData = new FormData(form);
        const params = Object.fromEntries(formData.entries());
        sendEvent(form.getAttribute("data-event"), params);
      });
    });
  }

  attachListeners();
})();
