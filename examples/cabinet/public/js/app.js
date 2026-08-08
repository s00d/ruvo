/* Layout Vue islands only (nav + flash). Page logic lives in each HTML {% block scripts %}. */
(function () {
  const Auth = window.SovaAuth || {};
  if (!Auth.mount) return;

  document.querySelectorAll("[data-vue]").forEach((el) => {
    const kind = el.getAttribute("data-vue");
    if (kind === "nav") {
      Auth.mount(el, {
        data() {
          return { open: false, logoutBusy: false };
        },
        mounted() {
          const path = location.pathname.replace(/\/$/, "") || "/";
          el.querySelectorAll("a.nav-link").forEach((a) => {
            const href = (a.getAttribute("href") || "").replace(/\/$/, "") || "/";
            if (href === path) {
              a.classList.add("text-mist-100", "font-semibold");
              a.classList.remove("text-mist-400");
            }
          });
        },
        methods: {
          async logout() {
            this.logoutBusy = true;
            try {
              await Auth.api.post("/logout");
              location.href = "/";
            } catch (e) {
              alert(Auth.errorMessage(e));
              this.logoutBusy = false;
            }
          },
        },
      });
    } else if (kind === "flash") {
      Auth.mount(el, {
        data() {
          return { visible: true };
        },
        methods: {
          dismiss() {
            this.visible = false;
          },
        },
      });
    }
  });
})();
