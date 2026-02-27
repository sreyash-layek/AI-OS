import { defineConfig } from "vite";

export default defineConfig({
  server: {
    host: true,
    port: 1420,
    strictPort: true,
    proxy: {
      "/health": "http://localhost:7777",
      "/v1": "http://localhost:7777"
    }
  }
});
