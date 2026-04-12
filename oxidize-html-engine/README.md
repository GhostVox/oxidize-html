# oxidize-html-engine

The core HTML rendering engine. Parses HTML, computes styles, performs layout, and emits draw commands. Has no dependency on any UI framework.

## Pipeline

```
HTML string
  → parser        (produces DOM via html5ever)
  → StyleEngine   (produces StyledNode tree)
  → LayoutEngine  (produces LayoutNode tree)
  → paint()       (produces Vec<DrawCommand>)
```

## Usage

Debug output (style tree and layout tree) can be enabled by passing `true` to the engine methods:

```rust
let mut renderer = HtmlRenderer::default();
let commands = renderer.render_html("<p>Hello</p>", 800.0);
```

By default debug output is disabled. To enable it during development, pass the debug flag through the engine directly:

```rust
let layout = engine.compute(&style_tree, 800.0, true);  // prints layout tree
let style = styler.compute(&dom, true);                  // prints style tree
```


`render_html` caches both the style tree and the layout tree. The style tree is recomputed only when the HTML changes. The layout tree is recomputed only when the HTML or available width changes.

## Draw Commands

The `Vec<DrawCommand>` returned by `render_html` is a flat list of drawing instructions:

- `FillRect` — filled rectangle (backgrounds)
- `StrokeRect` — bordered rectangle
- `DrawText` — text at a position
- `DrawImage` / `DrawImagePlaceholder` — images
- `DrawLine` — lines and borders
- `Link` — hit rect for a hyperlink

These commands are backend-agnostic. Use them with `oxidize-render` for GPUI, or implement your own renderer.

## Supported HTML

- Block elements: `div`, `p`, `h1`–`h3`, `ul`, `li`, `hr`
- Inline elements: `span`, `a`, `strong`, `b`, `em`, `i`, `small`, `br`
- Tables: `table`, `thead`, `tbody`, `tr`, `td`, `th` (including `colspan`)
- Images: local paths, remote URLs, data URIs, CID references
- Legacy: `font` tag color and size attributes

## Supported CSS

- Typography: `font-size`, `font-weight`, `font-style`, `font-family`, `line-height`, `color`, `text-align`, `text-decoration`
- Box model: `margin`, `padding`, `width`, `height`, `display`
- Backgrounds: `background-color`
- Borders: `border`, `border-top/right/bottom/left`
- Stylesheet rules via `<style>` blocks (tag, class, and compound selectors)
- Inline styles via the `style` attribute

## Running Tests

```sh
cargo test -p oxidize-html-engine
```