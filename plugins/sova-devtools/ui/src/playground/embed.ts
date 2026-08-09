import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "../App.vue";
import { router } from "../router";
import { useDevToolsStore } from "../stores/devtools";
import { mockBundle } from "./fixtures";
import "../style.css";

const el = document.getElementById("sova-devtools");
if (el) {
  const app = createApp(App);
  app.use(createPinia());
  app.use(router);
  app.mount(el);
  const store = useDevToolsStore();
  store.loadMock(mockBundle());
  store.open = true;
  void router.isReady().then(() => {
    store.boot();
  });
}

window.addEventListener("message", (ev) => {
  if (ev.data?.type === "sova-pg-reload") {
    location.reload();
  }
});
