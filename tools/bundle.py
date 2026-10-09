"""Inline the wasm-bindgen glue and the wasm binary into one HTML file."""
import base64
import sys

html_path, glue_path, wasm_path, out_path = sys.argv[1:5]
html = open(html_path, encoding="utf-8").read()
glue = open(glue_path, encoding="utf-8").read()
wasm_b64 = base64.b64encode(open(wasm_path, "rb").read()).decode("ascii")

glue_tag = '<script src="../pkg/qr_logo.js"></script>'
assert glue_tag in html, "glue script tag not found"
assert "</script>" not in glue
html = html.replace(glue_tag, "<script>\n" + glue + "\n</script>")

marker = "const WASM_B64 = null;"
assert marker in html, "wasm marker not found"
html = html.replace(marker, 'const WASM_B64 = "' + wasm_b64 + '";')

open(out_path, "w", encoding="utf-8").write(html)
