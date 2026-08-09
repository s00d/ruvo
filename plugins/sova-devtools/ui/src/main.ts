import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "./router";
import { useDevToolsStore } from "./stores/devtools";
import "./style.css";

const mount = document.getElementById("sova-devtools");
if (mount) {
  const app = createApp(App);
  app.use(createPinia());
  app.use(router);
  app.mount(mount);
  const store = useDevToolsStore();
  void router.isReady().then(() => {
    if (store.tab && router.currentRoute.value.name !== store.tab) {
      void router.replace({ name: store.tab });
    }
    store.boot();
  });
}
