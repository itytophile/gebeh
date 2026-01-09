import path, { dirname } from "path";
import CopyPlugin from "copy-webpack-plugin";
import WasmPackPlugin from "@wasm-tool/wasm-pack-plugin";
import { fileURLToPath } from "url";
import type { WebpackConfiguration } from "webpack-dev-server";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const dist = path.resolve(__dirname, "dist");

export default {
  experiments: { asyncWebAssembly: true },
  mode: "production",
  entry: {
    index: "./ts/index.ts",
  },
  output: {
    path: dist,
    filename: "[name].js",
  },
  devServer: {
    static: dist,
  },
  plugins: [
    new CopyPlugin({ patterns: [path.resolve(__dirname, "static")] }),

    new WasmPackPlugin({
      crateDirectory: __dirname,
    }),
  ],
} satisfies WebpackConfiguration;
