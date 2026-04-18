pub mod image;
pub mod layout;
pub mod painter;
pub mod parser;
pub mod styler;
pub mod table;

use layout::LayoutEngine;
use painter::paint;
use styler::StyleEngine;
// triggering workflow
pub type NodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Weight(u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    None,
    Table,
    TableRow,
    TableCell,
    ListItem,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
    Baseline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextDecoration {
    None,
    Underline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeValue {
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSpec {
    pub width: f32,
    pub color: Rgba,
}

/// The computed style of a DOM node, including all inherited styles.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    /// The computed color of the text.
    pub color: Rgba,
    /// The computed background color of the node.
    pub background_color: Option<Rgba>,
    /// The computed font size of the text.
    pub font_size: f32,
    /// The computed font weight of the text.
    pub font_weight: FontWeight,
    /// The computed font style of the text.
    pub font_style: FontStyle,
    /// The computed font family of the text.
    pub font_family: Vec<String>,
    /// The computed text alignment of the node.
    pub text_align: TextAlign,
    /// The computed line height of the text.
    pub line_height: f32,
    /// The computed padding of the node.
    pub padding: Edges<f32>,
    /// The computed margin of the node.
    pub margin: Edges<f32>,
    /// The computed width of the node.
    pub width: SizeValue,
    /// The computed height of the node.
    pub height: SizeValue,
    /// The computed display property of the node.
    pub display: Display,
    /// The computed vertical alignment of the node.
    pub vertical_align: VerticalAlign,
    /// The computed border of the node.
    pub border: Edges<BorderSpec>,
    /// The computed text decoration of the node.
    pub text_decoration: TextDecoration,
    /// The computed href of the link, if the node is a link.
    pub href: Option<String>,
}

impl Default for ComputedStyle {
    /// Creates a default computed style with default values.
    fn default() -> Self {
        Self {
            color: Rgba::rgb(0, 0, 0),
            background_color: None,
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_family: vec!["sans-serif".to_string()],
            text_align: TextAlign::Left,
            line_height: 19.2,
            padding: Edges::all(0.0),
            margin: Edges::all(0.0),
            width: SizeValue::Auto,
            height: SizeValue::Auto,
            display: Display::Block,
            vertical_align: VerticalAlign::Baseline,
            border: Edges::all(BorderSpec {
                width: 0.0,
                color: Rgba::rgb(0, 0, 0),
            }),
            text_decoration: TextDecoration::None,
            href: None,
        }
    }
}

/// A rectangle in 2D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// The x-coordinate of the top-left corner of the rectangle.
    pub x: f32,
    /// The y-coordinate of the top-left corner of the rectangle.
    pub y: f32,
    /// The width of the rectangle.
    pub width: f32,
    /// The height of the rectangle.
    pub height: f32,
}
impl Rect {
    /// Calculates the right edge of the rectangle.
    ///
    /// # Return
    /// f32 calculated by self.x + self.width.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Calculates the bottom edge of the rectangle.
    ///
    ///# Return
    /// f32 calculated by self.y + self.height.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }
}

/// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub lines: Vec<String>,
    pub line_height: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeContent {
    Text(TextLayout),
    Image {
        source: image::ImageSource,
        width: f32,
        height: f32,
    },
    Box,
    Hr,
}

/// A node in the DOM tree, with computed style and children.

#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode {
    /// Unique identifier for the node, used for caching and referencing in layout and paint stages.
    pub node_id: NodeId,
    /// The tag name of the node, e.g. "div", "p", "span", etc.
    pub tag: Option<String>,
    /// The attributes of the node, e.g. "id" and "class" attributes.
    pub attrs: std::collections::HashMap<String, String>,
    /// The text content of the node, if any.
    pub text: Option<String>,
    /// The
    pub style: ComputedStyle,
    pub children: Vec<StyledNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    pub node_id: NodeId,
    pub rect: Rect,
    pub style: ComputedStyle,
    pub content: NodeContent,
    pub bullet_origin: Option<Point>,
    pub children: Vec<LayoutNode>,
    pub tag: Option<String>,
}

pub type LayoutTree = LayoutNode;

/// A command to draw a shape or text on the screen
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    FillRect {
        rect: Rect,
        color: Rgba,
    },
    StrokeRect {
        rect: Rect,
        color: Rgba,
        width: f32,
    },
    DrawText {
        text: String,
        origin: Point,
        color: Rgba,
        font_size: f32,
    },
    DrawImagePlaceholder {
        rect: Rect,
    },
    DrawImage {
        rect: Rect,
        source: image::ImageSource,
    },
    DrawLine {
        start: Point,
        end: Point,
        color: Rgba,
        width: f32,
    },
    Link {
        rect: Rect,
        href: String,
    },
}

#[derive(Debug)]
pub struct HtmlRenderer {
    styler: StyleEngine,
    layout: LayoutEngine,
    last_width: f32,
    style_cache: Option<StyledNode>,
    layout_cache: Option<LayoutTree>,
    cached_html: String,
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self {
            styler: StyleEngine::default(),
            layout: LayoutEngine::default(),
            last_width: -1.0,
            style_cache: None,
            layout_cache: None,
            cached_html: String::new(),
        }
    }
}

impl HtmlRenderer {
    /// Renders the given HTML string into a list of draw commands to be used with GPUI to render HTML.
    pub fn render(&mut self, html: &str, available_width: f32, debug: bool) -> Vec<DrawCommand> {
        self.render_html(html, available_width, debug)
    }

    pub fn render_html(&mut self, html: &str, width: f32, debug: bool ) -> Vec<DrawCommand> {
        // Check if the HTML has changed since last render.
        let html_changed = self.cached_html != html;

        // If the HTML has changed or if the style cache is empty, recompute the dom and style tree.
        if self.style_cache.is_none() || html_changed {
            let dom = parser::parse(html);
            self.style_cache = Some(self.styler.compute(&dom, debug));
            self.cached_html = html.to_string();
            self.layout_cache = None;
            self.last_width = -1.0;
        }

        // If the width has changed, recompute the layout tree.
        let width_changed = (width - self.last_width).abs() > f32::EPSILON;
        if self.layout_cache.is_none() || width_changed {
            if let Some(base_style_tree) = &self.style_cache {
                let mut style_tree = base_style_tree.clone();
                table::normalize_tables(&mut style_tree, width);
                let layout_tree = self.layout.compute(&style_tree, width, debug);
                self.layout_cache = Some(layout_tree);
                self.last_width = width;
            }
        }

        // Take the layout tree and convert it to a list of draw commands.
        let mut commands = Vec::new();
        if let Some(layout_tree) = &self.layout_cache {
            paint(layout_tree, &mut commands);
        }
        commands
    }

    /// Parses the given HTML into a tree and returns the root node of a style tree.
    /// Only used in test functions
    pub fn style_tree(&mut self, html: &str) -> StyledNode {
        let dom = parser::parse(html);
        self.styler.compute(&dom,false )
    }
}

#[cfg(test)]
mod tests {
    use super::{Display, HtmlRenderer, SizeValue, StyledNode, TextAlign};

    fn find_first_tag<'a>(node: &'a StyledNode, tag: &str) -> Option<&'a StyledNode> {
        if node.tag.as_deref() == Some(tag) {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = find_first_tag(child, tag) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn font_tag_fallbacks_map_to_style() {
        let html = r##"<font color="#ff0000" size="5">hello</font>"##;
        let mut renderer = HtmlRenderer::default();
        let tree = renderer.style_tree(html);
        let font_node = find_first_tag(&tree, "font").expect("font node");
        assert_eq!(font_node.style.color.r, 255);
        assert!(matches!(font_node.style.display, Display::Inline));
        assert_eq!(font_node.style.font_size, 24.0);
    }

    #[test]
    fn td_attribute_width_and_alignment_are_resolved() {
        let html = r#"<table width="600"><tr><td width="200" align="center">X</td><td>Y</td></tr></table>"#;
        let mut renderer = HtmlRenderer::default();
        let tree = renderer.style_tree(html);
        let cell = find_first_tag(&tree, "td").expect("cell");
        assert!(matches!(cell.style.width, SizeValue::Px(200.0)));
        assert!(matches!(cell.style.text_align, TextAlign::Center));
    }
}
