## About

This directory shows how the `org-exporter` crate can be used via wasm.
It sets up an interactive org parsing environment where org source you type on the left is parsed and rendered on the right.
View it in action here: https://org-rust.pages.dev/

This project uses webpack to build and compile the dependencies for the site.
It's essentially just for [`codemirror`](https://codemirror.net/), which powers the editor.

### Installing

#### First build

First install [`wasm-pack`](https://github.com/rustwasm/wasm-pack), which will be used to compile the wasm module.
Then you'll need [npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) to install the web dependencies:

```
npm install
```


#### Subsequent builds


To compile the wasm module (will place the generated output in `/pkg`):

```
wasm-pack build  --release --target bundler
```

Then for the site itself (output will be found at `/dist`):

```
npm run deploy
```

or alternatively:

```
npm run demo
```

for dev builds.

`/dist` can then be served as is.

### Other

These are just some things of note:

- `iframe.html` is the file used to load in the iframe for rendering (it's technically not necessary? but using `src="about:blank"` causes some weird rendering issues).

- `webpack` was used over other bundlers due to the integration with `wasm-bindgen`. It allows "just importing" the things we need from `pkg/` without any shenanigans or plugins. 

- We use `index.html` as the entry point via the https://github.com/webdiscus/html-bundler-webpack-plugin plugin. It makes more sense this way since `index.html` is the entry point to the application.  See `webpack.config.js` for how.

- the `reparse()` function in `main.js` is throttled to prevent parsing too many times when parsing starts to take a while with extremely large input. 

- We maintain the same buffer on subsequent parses so that memory doesn't need to be continuously allocated/re-allocated. 




