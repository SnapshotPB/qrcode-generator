# QR Logo Generator

A QR code generator written in Rust and compiled to WebAssembly. The web page
works offline from a single HTML file. You give it a text and a logo image.
The logo colors show through the dots of the QR code.

## Build

Requirements: a Rust toolchain with the `wasm32-unknown-unknown` target,
`wasm-bindgen` 0.2.129, and Python 3. `wasm-opt` is optional.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
./build.sh
```

The build writes `dist/index.html`. Open this file in a browser. It has no
external resources and needs no server.

The `www/index.html` page is the source of the web page. It loads the module
from `pkg/`, so it needs a local HTTP server, for example `python3 -m http.server`
from the project root, then open `http://localhost:8000/www/`.

## Use

1. Type the text or URL.
2. Drop, paste or choose a logo image. The button **Use demo logo** loads a
   four-color ring.
3. Move the logo: drag it on the preview, use the arrow buttons, or use the
   position sliders. The arrow keys also move the logo when the focus is not in
   a field. Hold Shift for a quarter-module step.
4. Resize the logo: use the mouse wheel on the preview or the size slider.
   Hold Alt with the wheel to rotate the logo.
5. Download the result as SVG or PNG.

### Logo modes

- **Tint** (default): only the dark modules of the code take the logo color. The
  code keeps all its data, so the result scans with any logo size.
- **Fill**: every module under the logo becomes a dot in the logo color. The
  logo shape is drawn with dots, as in the example image. The data modules under
  the logo are lost, so the error correction must repair them. Keep the status
  line **Data modules changed** below the error correction limit (30 % for
  level H). Function patterns (finder, timing, alignment, format and version
  information) are never changed.

### Colorize

The option **Colorize the logo** gives every opaque logo pixel one color from
the color picker. Transparent pixels stay transparent, and white pixels stay
transparent when **Treat white as transparent** is on.

### Decode check

After each change the page rasterizes the result and decodes it with the
`rqrr` crate inside the WebAssembly module. The check uses the luminance of
each dot, so light logo colors count as light modules. The option
**Maximum brightness of logo colors** darkens light colors so the scanner reads
them as dark. The check is a good indicator, but always test the final image
with a phone camera.

## Project layout

- `src/qr.rs`: QR matrix from the `qrcode` crate and the function pattern mask.
- `src/render.rs`: logo sampling, module colors and SVG output.
- `src/verify.rs`: decode check with `rqrr`.
- `src/lib.rs`: the `wasm-bindgen` API (`RenderOptions`, `render`, `RenderResult`) and unit tests.
- `www/index.html`: the web page.
- `tools/bundle.py`: inlines the JavaScript glue and the wasm binary into `dist/index.html`.

## Test

```sh
cargo test
```
