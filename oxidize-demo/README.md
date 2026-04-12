# oxidize-demo

An interactive GPUI application for testing and developing `oxidize-html-engine`. Renders an HTML file in a resizable window and displays the source label and render width.

## Running

```sh
cargo run --bin gpui_demo -- path/to/file.html
```

If no file is provided, a built-in demo table is rendered.

## Test Files

| File | Description |
|------|-------------|
| `test-email.html` | A realistic HTML email with tables, inline styles, and BR tags |
| `test2.html` | Basic block layout, nested divs, and a 4-column table |

## Window Layout

The window is split into two panels:

- **Left** — the rendered HTML document, scrollable in both axes
- **Right** — metadata panel showing the source file and current render width

Resizing the window reflows the HTML layout automatically. Clicking links prints the href to stdout.