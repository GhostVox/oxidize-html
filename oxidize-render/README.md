# oxidize-render

GPUI rendering backend for `oxidize-html-engine`. Translates `DrawCommand`s into GPUI elements.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
oxidize-html-engine = { path = "../oxidize-html-engine" }
oxidize-render = { path = "../oxidize-render", features = ["gpui"] }
gpui = { git = "https://github.com/zed-industries/zed" }
```

Then in your GPUI component:

```rust
use oxidize_html_engine::HtmlRenderer;
use oxidize_render::gpui_renderer::{command_element, content_extent};
use gpui::{div, px};

let mut renderer = HtmlRenderer::default();
let commands = renderer.render_html(&html, width);
let (doc_width, doc_height) = content_extent(&commands);

let mut document = div()
    .relative()
    .w(px(doc_width))
    .h(px(doc_height));

for command in &commands {
    document = document.child(command_element(command));
}
```

## Public API

### `command_element(command: &DrawCommand) -> gpui::Div`
Converts a single `DrawCommand` into a GPUI `Div` element positioned absolutely within a document canvas.

### `content_extent(commands: &[DrawCommand]) -> (f32, f32)`
Computes the total `(width, height)` of the rendered document from its draw commands. Use this to size the canvas container.

### `to_bounds_with_offset(rect: Rect, ox: f32, oy: f32) -> Bounds<Pixels>`
Converts an engine `Rect` to a GPUI `Bounds<Pixels>` with a scroll or origin offset applied. Useful for hit testing links and interactive elements.
Example:

```rust
use oxidize_html_engine::DrawCommand;
use oxidize_render::gpui_renderer::to_bounds_with_offset;
use gpui::Bounds;

// inside Element::paint, where `bounds: Bounds` is available:
let ox = f32::from(bounds.origin.x);
let oy = f32::from(bounds.origin.y);
for command in &commands {
if let DrawCommand::Link { rect, href } = command {
let link_bounds = to_bounds_with_offset(*rect, ox, oy);
// use link_bounds for mouse hit testing
}
}
```

## Features

| Feature | Description |
|---------|-------------|
| `gpui` | Enables the GPUI rendering backend (required) |