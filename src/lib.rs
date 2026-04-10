pub mod image;
pub mod layout;
pub mod painter;
pub mod parser;
pub mod styler;
pub mod table;

use layout::LayoutEngine;
use painter::paint;
use styler::StyleEngine;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    pub color: Rgba,
    pub background_color: Option<Rgba>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_family: Vec<String>,
    pub text_align: TextAlign,
    pub line_height: f32,
    pub padding: Edges<f32>,
    pub margin: Edges<f32>,
    pub width: SizeValue,
    pub height: SizeValue,
    pub display: Display,
    pub vertical_align: VerticalAlign,
    pub border: Edges<BorderSpec>,
    pub text_decoration: TextDecoration,
    pub href: Option<String>,
}

impl Default for ComputedStyle {
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct StyledNode {
    pub node_id: NodeId,
    pub tag: Option<String>,
    pub attrs: std::collections::HashMap<String, String>,
    pub text: Option<String>,
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
}

pub type LayoutTree = LayoutNode;

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
    pub fn render(&mut self, html: &str, available_width: f32) -> Vec<DrawCommand> {
        self.render_html(html, available_width)
    }

    pub fn render_html(&mut self, html: &str, width: f32) -> Vec<DrawCommand> {
        let html_changed = self.cached_html != html;
        if self.style_cache.is_none() || html_changed {
            let dom = parser::parse(html);
            self.style_cache = Some(self.styler.compute(&dom));
            self.cached_html = html.to_string();
            self.layout_cache = None;
            self.last_width = -1.0;
        }

        let width_changed = (width - self.last_width).abs() > f32::EPSILON;
        if self.layout_cache.is_none() || width_changed {
            if let Some(base_style_tree) = &self.style_cache {
                let mut style_tree = base_style_tree.clone();
                table::normalize_tables(&mut style_tree, width);
                let layout_tree = self.layout.compute(&style_tree, width);
                self.layout_cache = Some(layout_tree);
                self.last_width = width;
            }
        }

        let mut commands = Vec::new();
        if let Some(layout_tree) = &self.layout_cache {
            paint(layout_tree, &mut commands);
        }
        commands
    }

    pub fn style_tree(&mut self, html: &str) -> StyledNode {
        let dom = parser::parse(html);
        self.styler.compute(&dom)
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
