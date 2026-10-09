"""Inline the wasm-bindgen glue and the wasm binary into one HTML file."""
import base64
import json
import sys

html_path, glue_path, wasm_path, logo_path, out_path = sys.argv[1:6]
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

logo_marker = "const DEMO_LOGO_SVG = null;"
assert logo_marker in html, "logo marker not found"
logo_svg = open(logo_path, encoding="utf-8").read()
html = html.replace(logo_marker, "const DEMO_LOGO_SVG = " + json.dumps(logo_svg) + ";")

open(out_path, "w", encoding="utf-8").write(html)
