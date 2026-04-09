# oxidize-mail: HTML Email Renderer Spec
> Target stack: Rust · GPUI · html5ever · taffy · parley · cosmic-text

---

## Goal

Build a self-contained `email-renderer` crate within the `oxidize-mail` workspace that takes a raw HTML email string and produces GPUI draw calls. No embedded browser. No WebKitGTK dependency. Covers ~90% of real-world email HTML.

---

## Crate placement

```
oxidize-mail/
  crates/
    core/
    gui/
    email-renderer/   ← new crate
      src/
        lib.rs
        parser.rs
        styler.rs
        layout.rs
        painter.rs
        image.rs
        table.rs
```

Add to workspace `Cargo.toml`:
```toml
[workspace]
members = ["crates/core", "crates/gui", "crates/email-renderer"]
```

---

## Dependencies

```toml
[dependencies]
html5ever       = "0.27"        # HTML parsing
markup5ever     = "0.12"        # DOM tree types (comes with html5ever)
taffy           = "0.5"         # Flexbox/block layout engine
parley          = "0.1"         # Text layout and shaping
cosmic-text     = "0.12"        # Font system / glyph rasterization
image           = "0.25"        # Image decoding (PNG, JPEG, GIF)
base64          = "0.22"        # Inline image decoding (data: URIs)
reqwest         = { version = "0.12", features = ["blocking"] }  # Remote image fetch
gpui            = { path = "../../vendor/gpui" }  # or git dep
```

---

## Pipeline overview

```
Raw HTML string
      │
      ▼
 [1] Parser          html5ever → DOM tree (RcDom)
      │
      ▼
 [2] Styler          Walk DOM, resolve inline styles → StyleTree
      │
      ▼
 [3] Table Pre-pass  Convert <table> structure → taffy-compatible flex nodes
      │
      ▼
 [4] Layout          taffy computes rects for all nodes
      │              parley computes text line breaks within text nodes
      ▼
 [5] Painter         Walk layout output → Vec<DrawCommand>
      │
      ▼
 [6] GPUI render     Consume DrawCommand list in GPUI's paint phase
```

---

## Step 1 — Parser (`parser.rs`)

**What it does:** Turn raw HTML bytes into a DOM tree.

**Implementation:**

```rust
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{RcDom, Handle};

pub fn parse(html: &str) -> RcDom {
    parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .unwrap()
}
```

**Notes:**
- `html5ever` is spec-compliant and handles malformed markup (which is the norm in email)
- Output is an `RcDom` — a reference-counted tree of `Handle` nodes
- No special configuration needed for email; default parsing options are fine

**Acceptance criteria:**
- Parses a Gmail-exported `.eml` HTML body without panicking
- Handles missing closing tags, implicit `<tbody>`, and mismatched nesting

---

## Step 2 — Styler (`styler.rs`)

**What it does:** Walk the DOM and produce a `StyleTree` — a parallel tree where each node has resolved CSS properties.

**Data structures:**

```rust
pub struct ComputedStyle {
    pub color: Rgba,
    pub background_color: Option<Rgba>,
    pub font_size: f32,          // px
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: Vec<String>,
    pub text_align: TextAlign,
    pub line_height: f32,        // px or multiplier
    pub padding: Edges<f32>,     // top/right/bottom/left in px
    pub margin: Edges<f32>,
    pub width: SizeValue,        // Auto, Px(f32), Percent(f32)
    pub height: SizeValue,
    pub display: Display,        // Block, Inline, InlineBlock, None, TableCell, etc.
    pub vertical_align: VerticalAlign,
    pub border: Edges<BorderSpec>,
    pub text_decoration: TextDecoration,
    pub href: Option<String>,    // pulled from <a href>
}
```

**CSS properties to support (email-relevant subset):**

| Property | Values |
|---|---|
| `color` | hex, rgb(), named colors |
| `background-color` | same |
| `font-size` | px, em, named (small/medium/large) |
| `font-weight` | normal, bold, 100–900 |
| `font-style` | normal, italic |
| `font-family` | comma-separated list |
| `text-align` | left, center, right |
| `line-height` | px, unitless multiplier |
| `padding` / `margin` | shorthand and longhand |
| `width` / `height` | px, %, auto |
| `display` | block, inline, inline-block, none |
| `border` | shorthand → width style color |
| `text-decoration` | none, underline |
| `vertical-align` | top, middle, bottom, baseline |

**Inheritance rules (apply to text nodes):**
- `color`, `font-*`, `text-align`, `line-height` inherit from parent
- `background-color`, `padding`, `margin`, `border`, `width`, `height` do not inherit

**HTML attribute fallbacks:**
- `<font color="">` → `color`
- `<font size="">` → `font-size` (HTML size 1–7 maps to px: 10, 13, 16, 18, 24, 32, 48)
- `<td bgcolor="">` → `background-color`
- `<td width="">` → `width`
- `<td align="">` → `text-align`
- `<td valign="">` → `vertical-align`
- `<img width="" height="">` → `width` / `height`

**Default styles by tag:**

```
h1: font-size 2em, font-weight bold, display block, margin 0.67em 0
h2: font-size 1.5em, font-weight bold, display block, margin 0.75em 0
h3: font-size 1.17em, font-weight bold, display block, margin 0.83em 0
p:  display block, margin 1em 0
b, strong: font-weight bold
i, em: font-style italic
a:  color #0000EE, text-decoration underline
ul: display block, margin 1em 0, padding-left 40px
li: display list-item
hr: display block, border-top: 1px solid #ccc, margin 0.5em 0
```

**Acceptance criteria:**
- A Gmail promotional email resolves correct font sizes and colors throughout
- `<font>` tags are treated identically to equivalent inline styles

---

## Step 3 — Table Pre-pass (`table.rs`)

**What it does:** Email uses `<table>` for layout. Taffy doesn't support CSS table layout natively. This pass converts table structure into taffy-compatible flex nodes before layout runs.

**Mapping:**

```
<table>  → taffy Column flex container (width from table style or 100%)
<tr>     → taffy Row flex container
<td>     → taffy flex item (flex: 0 0 auto, width from colspan/style)
<th>     → same as <td> but inherit font-weight: bold
```

**colspan/rowspan:**
- `colspan="2"` → set taffy node width to sum of column widths it spans
- `rowspan` → complex; for v1, treat rowspan > 1 as rowspan = 1 (known limitation, document it)

**Table width resolution:**
1. If `<table width="600">` or `style="width:600px"` → use that
2. If `width="100%"` → use available container width
3. If no width → shrink-wrap to content (taffy `fit_content`)

**Column width resolution:**
1. Collect explicit `<td width="">` or `style="width:"` values
2. If all columns have explicit widths → use them
3. If some columns lack widths → distribute remaining space equally among them
4. If no columns have widths → divide table width equally

**Acceptance criteria:**
- A 3-column promotional email with `<table width="600">` lays out columns at correct proportional widths
- Nested tables (table inside `<td>`) work recursively

---

## Step 4 — Layout (`layout.rs`)

**What it does:** Run taffy on the node tree (post table pre-pass) to compute `x, y, width, height` for every node. Run parley on text nodes to compute line breaks and glyph positions within their taffy-computed bounds.

**Taffy integration:**

```rust
use taffy::prelude::*;

pub struct LayoutEngine {
    taffy: TaffyTree,
    node_map: HashMap<NodeId, taffy::NodeId>,
}

impl LayoutEngine {
    pub fn compute(&mut self, root: &StyleNode, available_width: f32) -> LayoutTree {
        // 1. Walk StyleTree, create taffy nodes bottom-up
        // 2. Set taffy styles from ComputedStyle
        // 3. taffy.compute_layout(root_node, Size { width: available_width, height: auto })
        // 4. Walk taffy output, collect Layout { x, y, width, height } per node
    }
}
```

**taffy style mapping:**

```rust
Style {
    display: match computed.display {
        Display::Block => taffy::Display::Block,
        Display::Flex | Display::InlineBlock => taffy::Display::Flex,
        Display::None => taffy::Display::None,
        _ => taffy::Display::Block,
    },
    size: Size {
        width: match computed.width {
            SizeValue::Px(px) => Dimension::Length(px),
            SizeValue::Percent(p) => Dimension::Percent(p / 100.0),
            SizeValue::Auto => Dimension::Auto,
        },
        height: /* same pattern */,
    },
    padding: Rect {
        left: LengthPercentage::Length(computed.padding.left),
        right: LengthPercentage::Length(computed.padding.right),
        top: LengthPercentage::Length(computed.padding.top),
        bottom: LengthPercentage::Length(computed.padding.bottom),
    },
    // margin same pattern
    ..Default::default()
}
```

**Text layout with parley:**

For each text node, after taffy gives us a `width`:

```rust
use parley::{FontContext, LayoutContext};

pub fn layout_text(
    text: &str,
    style: &ComputedStyle,
    max_width: f32,
    font_ctx: &mut FontContext,
    layout_ctx: &mut LayoutContext,
) -> parley::Layout {
    let mut builder = layout_ctx.ranged_builder(font_ctx, text, 1.0);
    builder.push_default(&StyleProperty::FontSize(style.font_size));
    builder.push_default(&StyleProperty::FontWeight(style.font_weight.into()));
    builder.push_default(&StyleProperty::LineHeight(style.line_height));
    // ... other properties
    let mut layout = builder.build(text);
    layout.break_all_lines(Some(max_width));
    layout
}
```

**Output — `LayoutTree`:**

```rust
pub struct LayoutNode {
    pub node_id: NodeId,
    pub rect: Rect<f32>,          // absolute x, y, width, height
    pub style: ComputedStyle,
    pub content: NodeContent,
    pub children: Vec<LayoutNode>,
}

pub enum NodeContent {
    Text(parley::Layout),
    Image { src: ImageSource, width: f32, height: f32 },
    Box,   // div, td, etc. — just a rectangle
    Hr,
}
```

**Acceptance criteria:**
- Text wraps correctly at container boundary
- Nested divs produce correct absolute positions (child rect is offset by parent)
- `display: none` nodes are not included in output

---

## Step 5 — Painter (`painter.rs`)

**What it does:** Walk the `LayoutTree` and emit `DrawCommand`s. These are consumed in GPUI's `paint` method.

**Draw command enum:**

```rust
pub enum DrawCommand {
    FillRect {
        rect: gpui::Bounds<Pixels>,
        color: gpui::Rgba,
    },
    StrokeRect {
        rect: gpui::Bounds<Pixels>,
        color: gpui::Rgba,
        width: Pixels,
    },
    DrawText {
        layout: parley::Layout,
        origin: gpui::Point<Pixels>,
        color: gpui::Rgba,
    },
    DrawImage {
        rect: gpui::Bounds<Pixels>,
        image: Arc<gpui::Image>,
    },
    DrawLine {
        start: gpui::Point<Pixels>,
        end: gpui::Point<Pixels>,
        color: gpui::Rgba,
        width: Pixels,
    },
    PushClip(gpui::Bounds<Pixels>),
    PopClip,
    Link {
        rect: gpui::Bounds<Pixels>,
        href: String,
    },
}
```

**Paint walk:**

```rust
pub fn paint(node: &LayoutNode, commands: &mut Vec<DrawCommand>) {
    // 1. Background fill if background_color set
    if let Some(bg) = node.style.background_color {
        commands.push(DrawCommand::FillRect { rect: node.rect.into(), color: bg });
    }

    // 2. Borders
    paint_borders(node, commands);

    // 3. Content
    match &node.content {
        NodeContent::Box => {}   // background/border already handled
        NodeContent::Text(layout) => {
            commands.push(DrawCommand::DrawText {
                layout: layout.clone(),
                origin: node.rect.origin,
                color: node.style.color,
            });
        }
        NodeContent::Image { .. } => {
            // see image.rs
        }
        NodeContent::Hr => {
            commands.push(DrawCommand::DrawLine { /* ... */ });
        }
    }

    // 4. Link overlay
    if let Some(href) = &node.style.href {
        commands.push(DrawCommand::Link { rect: node.rect.into(), href: href.clone() });
    }

    // 5. Recurse children
    for child in &node.children {
        paint(child, commands);
    }
}
```

**Acceptance criteria:**
- Background colors fill exactly the taffy-computed rect
- Borders render on the correct edges at correct widths
- Link rects cover the full anchor element bounds

---

## Step 6 — Image loading (`image.rs`)

**What it does:** Resolve `<img src="">` values to decoded pixel data.

**Source types to handle:**

```rust
pub enum ImageSource {
    DataUri(Vec<u8>, ImageFormat),   // data:image/png;base64,...
    Remote(String),                   // https://...
    Cid(String),                      // cid:... (MIME attachment reference)
}
```

**Resolution:**

```rust
pub fn resolve_image(src: &str, mime_parts: &MimeParts) -> Option<ImageData> {
    if src.starts_with("data:") {
        resolve_data_uri(src)
    } else if src.starts_with("cid:") {
        resolve_cid(src, mime_parts)
    } else {
        // Remote: fetch async, return placeholder until loaded
        None  // caller schedules async fetch and triggers repaint
    }
}
```

**Remote image loading:**
- Fetch on a Tokio task via `CoreService` channel (same pattern as mail fetching)
- Render a grey placeholder rect until image arrives
- On arrival, send `MailEvent::ImageLoaded { id, data }` → trigger repaint

**Acceptance criteria:**
- `data:image/png;base64,...` inline images render correctly
- Remote images show a placeholder then render after fetch completes
- `cid:` references resolve against parsed MIME structure
- Broken/404 images render a placeholder without panicking

---

## GPUI integration

**In the GUI crate, inside your email view's `paint` implementation:**

```rust
impl Element for EmailBodyElement {
    fn paint(&mut self, bounds: Bounds<Pixels>, cx: &mut WindowContext) {
        let commands = self.renderer.paint_commands(bounds.size.width.0);
        for cmd in commands {
            match cmd {
                DrawCommand::FillRect { rect, color } => {
                    cx.paint_quad(gpui::fill(rect, color));
                }
                DrawCommand::DrawText { layout, origin, color } => {
                    // bridge parley layout → gpui text painting
                    paint_parley_layout(cx, &layout, origin, color);
                }
                DrawCommand::DrawImage { rect, image } => {
                    cx.paint_image(rect, image).ok();
                }
                DrawCommand::Link { rect, href } => {
                    cx.on_mouse_event(move |e: &MouseUpEvent, _, cx| {
                        if rect.contains(&e.position) {
                            cx.open_url(&href);
                        }
                    });
                }
                // ...
            }
        }
    }
}
```

**Note on parley → GPUI text bridge:**
This is the trickiest integration point. Parley produces glyph runs with font IDs and positions. GPUI has its own font system. Options:
1. Use `cosmic-text` for shaping and render glyphs manually via `cx.paint_glyph()` if GPUI exposes it
2. Convert parley output to GPUI's `ShapedLine` type if compatible
3. Fall back to GPUI's own text layout for simple runs, use parley only for complex reflow measurement

This bridge is the area most likely to need iteration — **treat it as the first spike to prototype.**

---

## Known limitations (v1 scope)

| Feature | Status |
|---|---|
| `rowspan > 1` | Not supported, treated as 1 |
| CSS class selectors | Not supported (email shouldn't use them) |
| `<style>` block parsing | Not supported in v1 |
| Animated GIFs | Static first frame only |
| Right-to-left text | Parley supports it; wire-up deferred |
| `background-image` | Not supported |
| `border-radius` | Not supported (GPUI may not expose rounded rects easily) |
| `<video>` / `<audio>` | Ignored |

---

## Milestones

### M1 — Parse + Style (no rendering)
- `parser.rs` parses real email HTML bodies
- `styler.rs` resolves inline styles and tag defaults
- Unit tests: feed 5 real emails, assert computed styles on known nodes

### M2 — Layout
- `table.rs` pre-pass converts tables to flex nodes
- `layout.rs` produces correct rects for a flat email (no nesting)
- Visual test: print ASCII art of layout rects to stdout

### M3 — Paint (boxes and text)
- `painter.rs` emits `DrawCommand` list
- GPUI view consumes commands and renders visible output
- Goal: a plain-text-heavy email (newsletter) renders readably

### M4 — Images
- `image.rs` handles `data:` URIs and remote fetches
- Promotional email with banner image renders correctly

### M5 — Polish
- `<style>` block parsing for the most common selectors
- `rowspan` support
- Link click handling
- Selection/copy (if GPUI supports it)

---

## Testing approach

Keep a `/test-emails/` directory of real `.html` files (export from Gmail/Proton/Thunderbird). Each milestone should render all of them without panicking. Visual correctness is eyeball-tested initially; automated pixel diff tests are a stretch goal.

```
test-emails/
  gmail-promotional.html
  github-notification.html
  outlook-corporate.html
  plain-text-heavy.html
  nested-tables.html
  inline-images.html
```
