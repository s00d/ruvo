import { createApp } from "vue";
import { createPinia } from "pinia";
import PlaygroundApp from "./PlaygroundApp.vue";
import "../style.css";

createApp(PlaygroundApp).use(createPinia()).mount("#pg-root");
