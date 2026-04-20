import "./assets/tailwind.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "vue-sonner/style.css";

import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { usePreferencesStore } from "./stores/preferences";

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);

// Apply saved font density before first render
const preferencesStore = usePreferencesStore();
preferencesStore.init();

app.mount("#app");
