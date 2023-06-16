## About

This directory shows how the `org-exporter` crate can be used via wasm.
It sets up an interactive org parsing environment where org code you type on the left is parsed and rendered on the right.
View it in action here: https://student.cs.uwaterloo.ca/~lbahodi/wasm

### Installing

First install [`wasm-pack`](https://github.com/rustwasm/wasm-pack), build with:

```
wasm-pack build  --release --target web
```

Then serve this directory and you'll be able to interact with the parser live.

### Other

- `iframe.html` is the file used to load in the iframe for rendering 

- We maintain the same buffer on subsequent parses so that memory doesn't need to be continuously allocated/re-allocated. 



