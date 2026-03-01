import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import typedCssModulesPlugin from "vite-plugin-typed-css-modules";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    typedCssModulesPlugin(),
    react({
      babel: {
        plugins: [["babel-plugin-react-compiler"]],
      },
    }),
  ],
  base: process.env.VITE_BASE,
});
