import { defineConfig } from "@playwright/test";

export default defineConfig({
  testMatch: "browser.spec.mjs",
  workers: 1,
  use: {
    headless: false,
    launchOptions: {
      args: [
        "--enable-unsafe-swiftshader",
        "--ignore-gpu-blocklist",
        "--use-angle=swiftshader",
      ],
    },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});
