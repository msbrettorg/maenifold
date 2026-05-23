import "./assets/tailwind.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "katex/dist/katex.min.css";
import "vue-sonner/style.css";

import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { usePreferencesStore } from "./stores/preferences";

const app = createApp(App);
const pinia = createPinia();

async function bootstrap(): Promise<void> {
  app.use(pinia);

  // Apply saved font density before first render
  const preferencesStore = usePreferencesStore();
  preferencesStore.init();

  app.mount("#app");

  if (import.meta.env.DEV) {
    const { installCdpPerfHarness } = await import("./lib/cdp-perf-harness");
    installCdpPerfHarness();
  }
}

void bootstrap();
