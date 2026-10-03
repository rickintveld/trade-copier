import { defineConfig, mergeConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

export default defineConfig((env) =>
  mergeConfig(viteConfig(env), {
    test: {
      environment: "jsdom",
      include: ["frontend/**/*.test.{ts,tsx}"],
      setupFiles: ["frontend/test/setup.ts"],
      // Timestamps in tests are asserted in UTC
      env: { TZ: "UTC" },
    },
  }),
);
