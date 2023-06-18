// import html from "@rollup/plugin-html"
// import { nodeResolve } from "@rollup/plugin-node-resolve"
// import wasm from "@rollup/plugin-wasm"
// export default {
//   input: "./editor.mjs",
//   output: {
//     dir: "./dist",
//     format: "es",
//   },

//   plugins: [html(),nodeResolve(), wasm({
//     fileName: "index.html"
//   })]
// }
import nodeResolve from '@rollup/plugin-node-resolve';
import wasm from '@rollup/plugin-wasm';
import { rollupPluginHTML as html } from '@web/rollup-plugin-html';
import { importMetaAssets } from '@web/rollup-plugin-import-meta-assets';
import copy from "rollup-plugin-copy-assets";

export default {
  input: 'index.html',
  output: { dir: 'dist' },
  plugins: [
    copy({
      assets: [
        "files/"
      ]
    }),
    html({ input: ['index.html', 'iframe.html'] }), wasm(), nodeResolve(),
  ],
};
