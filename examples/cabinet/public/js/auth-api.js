//! Axios + session helpers for Fortify JSON API (Laravel-style XSRF cookie).

(function (global) {
  function csrfMeta() {
    const meta = document.querySelector('meta[name="csrf-token"]');
    return (meta && meta.getAttribute("content")) || "";
  }

  const api = axios.create({
    baseURL: "/api/auth",
    withCredentials: true,
    xsrfCookieName: "XSRF-TOKEN",
    xsrfHeaderName: "X-XSRF-TOKEN",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
  });

  api.interceptors.request.use(function (config) {
    const t = csrfMeta();
    if (t && !config.headers["X-XSRF-TOKEN"] && !config.headers["X-CSRF-Token"]) {
      config.headers["X-CSRF-Token"] = t;
    }
    return config;
  });

  function errorMessage(err) {
    const data = err && err.response && err.response.data;
    if (!data) return (err && err.message) || "Request failed";
    if (typeof data === "string") return data;
    if (data.error) return typeof data.error === "string" ? data.error : JSON.stringify(data.error);
    if (data.errors) {
      const parts = [];
      Object.keys(data.errors).forEach(function (k) {
        const v = data.errors[k];
        parts.push(k + ": " + (Array.isArray(v) ? v.join(", ") : v));
      });
      return parts.join("; ") || "Validation failed";
    }
    if (data.message) return data.message;
    return "Request failed";
  }

  function formFields(el) {
    const data = {};
    el.querySelectorAll("input, select, textarea").forEach(function (input) {
      if (!input.name || input.type === "submit" || input.type === "button") return;
      if (input.type === "checkbox") {
        if (!data[input.name]) data[input.name] = [];
        if (input.checked) data[input.name].push(input.value);
        return;
      }
      const v = (input.value || "").trim();
      if (v === "") return;
      data[input.name] = input.value;
    });
    return data;
  }

  /** Mount a Vue island: SovaAuth.mount('#id', { data, methods, ... }). */
  function mount(selectorOrEl, options) {
    const { createApp } = global.Vue || {};
    if (!createApp) return null;
    const el =
      typeof selectorOrEl === "string"
        ? document.querySelector(selectorOrEl)
        : selectorOrEl;
    if (!el) return null;
    return createApp(options).mount(el);
  }

  global.SovaAuth = {
    api: api,
    csrfMeta: csrfMeta,
    errorMessage: errorMessage,
    formFields: formFields,
    mount: mount,
  };
})(window);
