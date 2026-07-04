(function () {
  const endpoint = window.location.pathname;
  const scriptTag = document.querySelector("script[data-model]");
  let model = scriptTag ? JSON.parse(scriptTag.getAttribute("data-model")) : null;

  function notifyModel() {
    if (window.maudliver && window.maudliver.onModel) {
      try {
        window.maudliver.onModel(model);
      } catch (e) {}
    }
  }

  function sendEvent(eventName, params) {
    fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ event: eventName, model: model, params: params || {} }),
    })
      .then(function (res) {
        return res.json();
      })
      .then(function (data) {
        if (data.redirect) {
          window.location.href = data.redirect;
          return;
        }
        data.patches.forEach(function (patch) {
          var el = document.getElementById(patch.id);
          if (el) el.outerHTML = patch.html;
        });
        model = data.model;
        notifyModel();
      });
  }

  // Event delegation on document — survives DOM replacement from patches, so
  // there's no need to re-attach listeners (which would double-bind and fire
  // events twice on elements that weren't replaced).
  document.addEventListener("click", function (e) {
    var el = e.target.closest("[data-event]");
    if (!el || el.tagName === "FORM" || el.closest("form[data-event]")) return;
    e.preventDefault();
    var params = {};
    Array.prototype.forEach.call(el.attributes, function (attr) {
      if (attr.name.indexOf("data-param-") === 0) {
        params[attr.name.slice("data-param-".length)] = attr.value;
      }
    });
    sendEvent(el.getAttribute("data-event"), params);
  });

  document.addEventListener("submit", function (e) {
    var form = e.target.closest("form[data-event]");
    if (!form) return;
    e.preventDefault();
    var params = {};
    new FormData(form).forEach(function (value, key) {
      params[key] = value;
    });
    sendEvent(form.getAttribute("data-event"), params);
  });

  // Exposed so app.js can persist/restore model state (e.g. tree expansion).
  window.maudliver = {
    send: sendEvent,
    onModel: null,
    get model() {
      return model;
    },
  };
})();
