import "./assets/tailwind.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "vue-sonner/style.css";
import "highlight.js/styles/github-dark.css";

import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { usePreferencesStore } from "./stores/preferences";
import { configureMarkdown } from "./lib/markdown";

// Wire highlight.js into the singleton `marked` instance so fenced code
// blocks render with syntax highlighting in chat / thinking views.
configureMarkdown();

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);

// Apply saved font density before first render
const preferencesStore = usePreferencesStore();
preferencesStore.init();

app.mount("#app");
