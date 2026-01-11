import { dirname, resolve } from "path";
import CopyPlugin from "copy-webpack-plugin";
import WasmPackPlugin from "@wasm-tool/wasm-pack-plugin";
import { fileURLToPath } from "url";
import type { WebpackConfiguration } from "webpack-dev-server";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const dist = resolve(__dirname, "dist");

export default {
  experiments: { asyncWebAssembly: true },
  devtool: "inline-source-map",
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
  // https://github.com/TypeStrong/ts-loader/blob/f7d022f79d1dae3c0c07ee63ec63c697eb99b32a/examples/vanilla/webpack.config.js
  module: {
    rules: [
      {
        test: /\.([cm]?ts|tsx)$/,
        loader: "ts-loader",
      },
    ],
  },
  resolve: {
    extensions: [".ts", ".tsx", ".js"],
    extensionAlias: {
      ".ts": [".js", ".ts"],
      ".cts": [".cjs", ".cts"],
      ".mts": [".mjs", ".mts"],
    },
  },
  plugins: [
    new CopyPlugin({ patterns: [resolve(__dirname, "static")] }),

    new WasmPackPlugin({
      crateDirectory: __dirname,
    }),
  ],
} satisfies WebpackConfiguration;
