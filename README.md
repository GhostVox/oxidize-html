# oxidize-html

A Rust workspace for parsing, styling, laying out, and rendering HTML — built for use in native GUI applications.

[![crates.io](https://img.shields.io/crates/v/oxidize-html.svg)](https://crates.io/crates/oxidize-html)
[![docs.rs](https://img.shields.io/docsrs/oxidize-html)](https://docs.rs/oxidize-html)
[![crates.io](https://img.shields.io/crates/v/oxidize-html-gpui.svg)](https://crates.io/crates/oxidize-html-gpui)
[![docs.rs](https://img.shields.io/docsrs/oxidize-html-gpui)](https://docs.rs/oxidize-html-gpui)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Crates

| Crate | Description |
|-------|-------------|
| `oxidize-html-engine` | Core HTML parser, style engine, layout engine, and painter |
| `oxidize-render` | GPUI rendering backend — translates draw commands into GPUI elements |
| `oxidize-demo` | Interactive demo app built with GPUI for testing and development |

## Quick Start

```sh
cd oxidize-demo
cargo run --bin gpui_demo -- test-email.html
```

## Workspace Structure

```
oxidize-html/
├── oxidize-html-engine/   # Core engine (no UI dependencies)
├── oxidize-render/        # GPUI rendering glue
└── oxidize-demo/          # Demo application
```

