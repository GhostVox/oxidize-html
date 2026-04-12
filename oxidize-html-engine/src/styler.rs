use std::collections::HashMap;

use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::{
    BorderSpec, ComputedStyle, Display, Edges, FontStyle, FontWeight, NodeId, Rgba, SizeValue,
    StyledNode, TextAlign, TextDecoration, VerticalAlign,
};

#[derive(Debug, Clone, PartialEq)]
struct CssRule {
    selectors: Vec<String>,
    declarations: Vec<(String, String)>,
}

#[derive(Default, Debug)]
pub struct StyleEngine {
    next_id: NodeId,
    rules: Vec<CssRule>,
}

impl StyleEngine {
    pub fn compute(&mut self, dom: &RcDom, debug: bool) -> StyledNode {
        self.next_id = 0;
        self.rules = Self::collect_stylesheet_rules(dom);
        let root_style = ComputedStyle::default();

        let root = self.visit(&dom.document, &root_style);
        if debug {
            println!("Debug style tree:");
            println!();
            print_style_tree(&root, 0);
        }
        root
    }

    fn visit(&mut self, handle: &Handle, inherited: &ComputedStyle) -> StyledNode {
        let node_id = self.alloc_id(); // Predictable ID allocation

        let mut style = inherited.clone();
        let mut tag = None;
        let mut attrs = HashMap::new();
        let mut text = None;

        match &handle.data {
            NodeData::Document => {
                // The root MUST be visible to process children
                style.display = Display::Block;
            }
            NodeData::Element {
                name, attrs: raw, ..
            } => {
                let tag_name = name.local.to_string().to_ascii_lowercase();
                tag = Some(tag_name.clone());
                for a in raw.borrow().iter() {
                    attrs.insert(
                        a.name.local.to_string().to_ascii_lowercase(),
                        a.value.to_string(),
                    );
                }

                style = base_style_with_inheritance(inherited);
                apply_tag_defaults(&tag_name, &mut style, inherited.font_size);
                self.apply_stylesheet_rules(&tag_name, &attrs, &mut style);
                apply_attribute_fallbacks(&tag_name, &attrs, &mut style, inherited.font_size);
                if let Some(inline) = attrs.get("style") {
                    apply_inline_style(inline, &mut style, inherited.font_size);
                }
            }
            NodeData::Text { contents } => {
                let value = contents.borrow().to_string().replace('\u{00A0}', " ");
                style.display = Display::Inline;
                if !value.trim().is_empty() {
                    text = Some(value);
                }
            }
            _ => {
                // Comments and Doctypes should be ignored
                style.display = Display::None;
            }
        }

        let mut children = Vec::new();
        for child in handle.children.borrow().iter() {
            let child_node = self.visit(child, &style);

            if child_node.style.display != Display::None
                && (child_node.text.is_some()
                    || child_node.tag.is_some()
                    || !child_node.children.is_empty())
            {
                children.push(child_node);
            }
        }

        StyledNode {
            node_id,
            tag,
            attrs,
            text,
            style,
            children,
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn collect_stylesheet_rules(dom: &RcDom) -> Vec<CssRule> {
        let mut css = String::new();
        collect_style_text(&dom.document, &mut css);
        parse_stylesheet(&css)
    }

    fn apply_stylesheet_rules(
        &self,
        tag: &str,
        attrs: &HashMap<String, String>,
        style: &mut ComputedStyle,
    ) {
        for rule in &self.rules {
            if !rule
                .selectors
                .iter()
                .any(|s| selector_matches(s, tag, attrs))
            {
                continue;
            }
            let parent_font_size = style.font_size;
            for (key, value) in &rule.declarations {
                apply_style_declaration(key, value, style, parent_font_size);
            }
        }
    }
}

fn collect_style_text(handle: &Handle, out: &mut String) {
    if let NodeData::Element { name, .. } = &handle.data
        && name.local.as_ref().eq_ignore_ascii_case("style")
    {
        for child in handle.children.borrow().iter() {
            if let NodeData::Text { contents } = &child.data {
                out.push_str(&contents.borrow());
                out.push('\n');
            }
        }
    }
    for child in handle.children.borrow().iter() {
        collect_style_text(child, out);
    }
}

fn parse_stylesheet(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    for block in css.split('}') {
        let Some((selector_part, declarations_part)) = block.split_once('{') else {
            continue;
        };
        let selectors: Vec<String> = selector_part
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if selectors.is_empty() {
            continue;
        }
        let declarations: Vec<(String, String)> = declarations_part
            .split(';')
            .filter_map(|d| {
                let (k, v) = d.split_once(':')?;
                let key = k.trim().to_ascii_lowercase();
                let value = v.trim().to_string();
                if key.is_empty() || value.is_empty() {
                    None
                } else {
                    Some((key, value))
                }
            })
            .collect();
        rules.push(CssRule {
            selectors,
            declarations,
        });
    }
    rules
}

fn selector_matches(selector: &str, tag: &str, attrs: &HashMap<String, String>) -> bool {
    let selector = selector.trim().to_ascii_lowercase();
    if selector == tag {
        return true;
    }
    if let Some(class_name) = selector.strip_prefix('.') {
        return has_class(attrs, class_name);
    }
    if let Some((sel_tag, class_name)) = selector.split_once('.') {
        return sel_tag == tag && has_class(attrs, class_name);
    }
    false
}

fn has_class(attrs: &HashMap<String, String>, class_name: &str) -> bool {
    attrs
        .get("class")
        .map(|classes| {
            classes
                .split_whitespace()
                .any(|c| c.eq_ignore_ascii_case(class_name))
        })
        .unwrap_or(false)
}

fn base_style_with_inheritance(parent: &ComputedStyle) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    // 1. Inherit Typography and Color
    style.color = parent.color;
    style.font_size = parent.font_size;
    style.font_weight = parent.font_weight;
    style.font_style = parent.font_style;
    style.font_family = parent.font_family.clone();
    style.line_height = parent.line_height;

    // 2. Inherit Alignment
    style.text_align = parent.text_align;
    style.vertical_align = parent.vertical_align;
    style
}

fn apply_tag_defaults(tag: &str, style: &mut ComputedStyle, parent_font_size: f32) {
    match tag {
        // Invisible metadata tags
        "head" | "meta" | "title" | "script" | "style" | "link" => {
            style.display = Display::None;
        }

        "br" => {
            style.display = Display::Inline;
        }

        // Table structural wrappers
        "tbody" | "thead" | "tfoot" => {
            style.display = Display::Block;
            style.margin = Edges::all(0.0);
            style.padding = Edges::all(0.0);
        }

        // Block and Table elements
        "html" | "body" | "div" | "section" | "article" | "table" | "tr" | "td" | "th" => {
            style.display = match tag {
                "table" => Display::Table,
                "tr" => Display::TableRow,
                "td" | "th" => Display::TableCell,
                _ => Display::Block,
            };

            style.text_align = TextAlign::Left;

            if matches!(tag, "td" | "th") {
                style.vertical_align = VerticalAlign::Middle;
                if tag == "th" {
                    style.font_weight = FontWeight::Bold;
                }
            }

            if matches!(tag, "html" | "body" | "table" | "tr") {
                style.margin = Edges::all(0.0);
                style.padding = Edges::all(0.0);
            }
        }

        "span" | "font" => style.display = Display::Inline,

        "a" => {
            style.display = Display::Inline;
            style.color = parse_color("#0000EE").unwrap_or(style.color);
            style.text_decoration = TextDecoration::Underline;
        }

        "b" | "strong" => {
            style.display = Display::Inline;
            style.font_weight = FontWeight::Bold;
        }

        "i" | "em" => {
            style.display = Display::Inline;
            style.font_style = FontStyle::Italic;
        }

        "u" | "ins" => {
            style.display = Display::Inline;
            style.text_decoration = TextDecoration::Underline;
        }

        "h1" => {
            style.display = Display::Block; // CRITICAL: Was missing
            style.font_size = parent_font_size * 2.0;
            style.font_weight = FontWeight::Bold;
            style.line_height = style.font_size * 1.2;
            style.margin = Edges::all(0.0);
            style.margin.top = style.font_size * 0.67;
            style.margin.bottom = style.font_size * 0.67;
        }

        "h2" => {
            style.display = Display::Block;
            style.font_size = parent_font_size * 1.5;
            style.font_weight = FontWeight::Bold;
            style.line_height = style.font_size * 1.2; // Added for safety
            style.margin = Edges::all(0.0);
            style.margin.top = parent_font_size * 0.75;
            style.margin.bottom = parent_font_size * 0.75;
        }

        "h3" => {
            style.display = Display::Block;
            style.font_size = parent_font_size * 1.17;
            style.font_weight = FontWeight::Bold;
            style.line_height = style.font_size * 1.2; // Added for safety
            style.margin = Edges::all(0.0);
            style.margin.top = parent_font_size * 0.83;
            style.margin.bottom = parent_font_size * 0.83;
        }

        "p" | "ul" => {
            style.display = Display::Block;
            style.margin = Edges::all(0.0);
            style.margin.top = parent_font_size;
            style.margin.bottom = parent_font_size;
            if tag == "ul" {
                style.padding.left = 40.0;
            }
        }

        "li" => {
            style.display = Display::ListItem;
        }

        "hr" => {
            style.display = Display::Block;
            style.margin = Edges::all(0.0);
            style.margin.top = parent_font_size * 0.5;
            style.margin.bottom = parent_font_size * 0.5;
            style.border.top = BorderSpec {
                width: 1.0,
                color: Rgba::rgb(204, 204, 204),
            };
        }

        "img" => style.display = Display::InlineBlock,
        "small" => {
            style.display = Display::Inline;
            style.font_size = parent_font_size * 0.875;
        }
        "sub" | "sup" => {
            style.display = Display::Inline;
            style.font_size = parent_font_size * 0.75;
        }
        _ => {}
    }
}


fn apply_attribute_fallbacks(
    tag: &str,
    attrs: &HashMap<String, String>,
    style: &mut ComputedStyle,
    parent_font_size: f32,
) {
    match tag {
        "font" => {
            if let Some(color) = attrs.get("color").and_then(|v| parse_color(v)) {
                style.color = color;
            }
            if let Some(size) = attrs.get("size").and_then(|v| parse_html_font_size(v)) {
                style.font_size = size;
                style.line_height = size * 1.2;
            }
        }
        "td" | "th" => {
            if let Some(bg) = attrs.get("bgcolor").and_then(|v| parse_color(v)) {
                style.background_color = Some(bg);
            }
            if let Some(width) = attrs
                .get("width")
                .and_then(|v| parse_size(v, parent_font_size))
            {
                style.width = width;
            }
            if let Some(align) = attrs.get("align").and_then(|v| parse_text_align(v)) {
                style.text_align = align;
            }
            if let Some(valign) = attrs.get("valign").and_then(|v| parse_vertical_align(v)) {
                style.vertical_align = valign;
            }
            if tag == "th" {
                style.font_weight = FontWeight::Bold;
            }
        }
        "img" => {
            if let Some(width) = attrs
                .get("width")
                .and_then(|v| parse_size(v, parent_font_size))
            {
                style.width = width;
            }
            if let Some(height) = attrs
                .get("height")
                .and_then(|v| parse_size(v, parent_font_size))
            {
                style.height = height;
            }
        }
        "a" => {
            if let Some(href) = attrs.get("href") {
                style.href = Some(href.clone());
            }
        }
        _ => {}
    }
}

fn apply_inline_style(input: &str, style: &mut ComputedStyle, parent_font_size: f32) {
    for declaration in input.split(';') {
        let mut kv = declaration.splitn(2, ':');
        let key = kv
            .next()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase();
        let value = kv.next().map(str::trim).unwrap_or_default();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        apply_style_declaration(&key, value, style, parent_font_size);
    }
    if style.line_height <= 0.0 {
        style.line_height = style.font_size * 1.2;
    }
}

fn apply_style_declaration(
    key: &str,
    value: &str,
    style: &mut ComputedStyle,
    parent_font_size: f32,
) {
    match key {
        "color" => {
            if let Some(color) = parse_color(value) {
                style.color = color;
            }
        }
        "background-color" => {
            style.background_color = parse_color(value);
        }
        "font-size" => {
            if let Some(px) = parse_font_size(value, parent_font_size) {
                style.font_size = px;
            }
        }
        "font-weight" => {
            if let Some(w) = parse_font_weight(value) {
                style.font_weight = w;
            }
        }
        "font-style" => {
            if value.eq_ignore_ascii_case("italic") {
                style.font_style = FontStyle::Italic;
            }
        }
        "font-family" => {
            style.font_family = value
                .split(',')
                .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|v| !v.is_empty())
                .collect();
        }
        "text-align" => {
            if let Some(align) = parse_text_align(value) {
                style.text_align = align;
            }
        }
        "line-height" => {
            if let Some(line_height) = parse_line_height(value, style.font_size) {
                style.line_height = line_height;
            }
        }
        "padding" => style.padding = parse_edge_shorthand(value, parent_font_size),
        "padding-top" => {
            style.padding.top =
                parse_length_like(value, parent_font_size).unwrap_or(style.padding.top)
        }
        "padding-right" => {
            style.padding.right =
                parse_length_like(value, parent_font_size).unwrap_or(style.padding.right)
        }
        "padding-bottom" => {
            style.padding.bottom =
                parse_length_like(value, parent_font_size).unwrap_or(style.padding.bottom)
        }
        "padding-left" => {
            style.padding.left =
                parse_length_like(value, parent_font_size).unwrap_or(style.padding.left)
        }
        "margin" => style.margin = parse_edge_shorthand(value, parent_font_size),
        "margin-top" => {
            style.margin.top =
                parse_length_like(value, parent_font_size).unwrap_or(style.margin.top)
        }
        "margin-right" => {
            style.margin.right =
                parse_length_like(value, parent_font_size).unwrap_or(style.margin.right)
        }
        "margin-bottom" => {
            style.margin.bottom =
                parse_length_like(value, parent_font_size).unwrap_or(style.margin.bottom)
        }
        "margin-left" => {
            style.margin.left =
                parse_length_like(value, parent_font_size).unwrap_or(style.margin.left)
        }
        "width" => style.width = parse_size(value, parent_font_size).unwrap_or(style.width),
        "height" => style.height = parse_size(value, parent_font_size).unwrap_or(style.height),
        "display" => style.display = parse_display(value).unwrap_or(style.display),
        "vertical-align" => {
            if let Some(va) = parse_vertical_align(value) {
                style.vertical_align = va;
            }
        }
        "text-decoration" => {
            style.text_decoration = if value.eq_ignore_ascii_case("underline") {
                TextDecoration::Underline
            } else {
                TextDecoration::None
            };
        }
        "border" => apply_border_shorthand(value, style, parent_font_size),
        "border-top" => {
            style.border.top =
                parse_border_spec(value, parent_font_size).unwrap_or(style.border.top)
        }
        "border-right" => {
            style.border.right =
                parse_border_spec(value, parent_font_size).unwrap_or(style.border.right)
        }
        "border-bottom" => {
            style.border.bottom =
                parse_border_spec(value, parent_font_size).unwrap_or(style.border.bottom)
        }
        "border-left" => {
            style.border.left =
                parse_border_spec(value, parent_font_size).unwrap_or(style.border.left)
        }
        _ => {}
    }
}

fn parse_edge_shorthand(value: &str, base_font_size: f32) -> Edges<f32> {
    let nums: Vec<f32> = value
        .split_whitespace()
        .filter_map(|token| parse_length_like(token, base_font_size))
        .collect();

    match nums.as_slice() {
        [v] => Edges::all(*v),
        [v1, v2] => Edges {
            top: *v1,
            right: *v2,
            bottom: *v1,
            left: *v2,
        },
        [v1, v2, v3] => Edges {
            top: *v1,
            right: *v2,
            bottom: *v3,
            left: *v2,
        },
        [v1, v2, v3, v4] => Edges {
            top: *v1,
            right: *v2,
            bottom: *v3,
            left: *v4,
        },
        _ => Edges::all(0.0),
    }
}

fn apply_border_shorthand(value: &str, style: &mut ComputedStyle, base_font_size: f32) {
    if let Some(spec) = parse_border_spec(value, base_font_size) {
        style.border = Edges::all(spec);
    }
}

fn parse_border_spec(value: &str, base_font_size: f32) -> Option<BorderSpec> {
    let mut width = None;
    let mut color = None;
    for token in value.split_whitespace() {
        if width.is_none() {
            width = parse_length_like(token, base_font_size);
        }
        if color.is_none() {
            color = parse_color(token);
        }
    }
    let width = width.unwrap_or(0.0);
    let color = color.unwrap_or(Rgba::rgb(0, 0, 0));
    if width > 0.0 {
        Some(BorderSpec { width, color })
    } else {
        None
    }
}

fn parse_display(value: &str) -> Option<Display> {
    match value.trim().to_ascii_lowercase().as_str() {
        "block" => Some(Display::Block),
        "inline" => Some(Display::Inline),
        "inline-block" => Some(Display::InlineBlock),
        "none" => Some(Display::None),
        _ => None,
    }
}

fn parse_html_font_size(value: &str) -> Option<f32> {
    match value.trim().parse::<u8>().ok()? {
        1 => Some(10.0),
        2 => Some(13.0),
        3 => Some(16.0),
        4 => Some(18.0),
        5 => Some(24.0),
        6 => Some(32.0),
        7 => Some(48.0),
        _ => None,
    }
}

fn parse_font_size(value: &str, parent_font_size: f32) -> Option<f32> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }

    match value.as_str() {
        "xx-small" => Some(9.0),
        "x-small" => Some(10.0),
        "small" => Some(13.0),
        "medium" => Some(16.0),
        "large" => Some(18.0),
        "x-large" => Some(24.0),
        "xx-large" => Some(32.0),
        _ if value.ends_with("px") => value.trim_end_matches("px").trim().parse().ok(),
        _ if value.ends_with("pt") => {
            // 1pt is roughly 1.33px
            let pt: f32 = value.trim_end_matches("pt").trim().parse().ok()?;
            Some(pt * 1.333)
        }
        _ if value.ends_with("em") || value.ends_with("rem") => {
            let factor: f32 = value
                .trim_end_matches("rem")
                .trim_end_matches("em")
                .trim()
                .parse()
                .ok()?;
            Some(parent_font_size * factor)
        }
        _ if value.ends_with('%') => {
            let pct: f32 = value.trim_end_matches('%').trim().parse().ok()?;
            Some(parent_font_size * (pct / 100.0))
        }
        _ => value.parse::<f32>().ok(),
    }
}

fn parse_font_weight(value: &str) -> Option<FontWeight> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("normal") {
        return Some(FontWeight::Normal);
    }
    if value.eq_ignore_ascii_case("bold") {
        return Some(FontWeight::Bold);
    }
    value.parse::<u16>().ok().map(FontWeight::Weight)
}

fn parse_text_align(value: &str) -> Option<TextAlign> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" => Some(TextAlign::Right),
        _ => None,
    }
}

fn parse_vertical_align(value: &str) -> Option<VerticalAlign> {
    match value.trim().to_ascii_lowercase().as_str() {
        "top" => Some(VerticalAlign::Top),
        "middle" => Some(VerticalAlign::Middle),
        "bottom" => Some(VerticalAlign::Bottom),
        "baseline" => Some(VerticalAlign::Baseline),
        _ => None,
    }
}

fn parse_line_height(value: &str, font_size: f32) -> Option<f32> {
    let value = value.trim().to_ascii_lowercase();
    if value.ends_with("px") {
        value.trim_end_matches("px").parse().ok()
    } else {
        value.parse::<f32>().ok().map(|multiplier| {
            if multiplier <= 4.0 {
                multiplier * font_size
            } else {
                multiplier
            }
        })
    }
}

fn parse_size(value: &str, base_font_size: f32) -> Option<SizeValue> {
    let value = value.trim().to_ascii_lowercase();
    if value == "auto" {
        return Some(SizeValue::Auto);
    }
    if value.ends_with('%') {
        return value
            .trim_end_matches('%')
            .parse::<f32>()
            .ok()
            .map(SizeValue::Percent);
    }
    parse_length_like(&value, base_font_size).map(SizeValue::Px)
}

fn print_style_tree(node: &StyledNode, indent: usize) {
    let indent_str = "  ".repeat(indent);

    // 1. Format the identity of the node
    if let Some(tag) = &node.tag {
        // Use brackets for tags to make them stand out
        print!("{}[<{}>]", indent_str, tag);
    } else if let Some(text) = &node.text {
        // Truncate long text so it doesn't wrap and break the visual tree
        let truncated: String = text.chars().take(40).collect();
        let display_text = if text.len() > 40 {
            format!("{}...", truncated)
        } else {
            truncated
        };
        print!("{}\"{}\"", indent_str, display_text.escape_debug());
    } else {
        print!("{}<anonymous>", indent_str);
    }

    // 2. Print only the "interesting" style properties
    let s = &node.style;
    let mut props = Vec::new();

    if s.display != Display::Block {
        props.push(format!("display:{:?}", s.display));
    }
    if let Some(bg) = s.background_color {
        props.push(format!("bg:{:?}", bg));
    }

    // Only show margins/padding if they aren't zero
    if s.margin.top != 0.0
        || s.margin.bottom != 0.0
        || s.margin.left != 0.0
        || s.margin.right != 0.0
    {
        props.push(format!("margin:{:?}", s.margin));
    }

    if s.font_size != 16.0 {
        props.push(format!("size:{}", s.font_size));
    }
    if s.font_weight != FontWeight::Normal {
        props.push("bold".to_string());
    }

    if !props.is_empty() {
        print!(" \x1b[33m-- {}\x1b[0m", props.join(", ")); // Yellow color for styles
    }

    println!(); // End line

    // 3. Recurse
    for child in &node.children {
        print_style_tree(child, indent + 1);
    }
}

fn parse_length_like(value: &str, base_font_size: f32) -> Option<f32> {
    let value = value.trim().to_ascii_lowercase();
    if value.ends_with("px") {
        value.trim_end_matches("px").parse().ok()
    } else if value.ends_with("em") {
        let em = value.trim_end_matches("em").parse::<f32>().ok()?;
        Some(em * base_font_size)
    } else {
        value.parse::<f32>().ok()
    }
}

pub fn parse_color(value: &str) -> Option<Rgba> {
    let value = value.trim().to_ascii_lowercase();
    if value.starts_with('#') {
        return parse_hex_color(&value);
    }
    if value.starts_with("rgb(") && value.ends_with(')') {
        let parts: Vec<u8> = value
            .trim_start_matches("rgb(")
            .trim_end_matches(')')
            .split(',')
            .filter_map(|p| p.trim().parse::<u8>().ok())
            .collect();
        if let [r, g, b] = parts.as_slice() {
            return Some(Rgba::rgb(*r, *g, *b));
        }
    }
    named_color(&value)
}

fn parse_hex_color(value: &str) -> Option<Rgba> {
    let hex = value.trim_start_matches('#');
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Rgba::rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Rgba::rgb(r, g, b))
        }
        _ => None,
    }
}

fn named_color(value: &str) -> Option<Rgba> {
    let color = match value {
        "black" => Rgba::rgb(0, 0, 0),
        "white" => Rgba::rgb(255, 255, 255),
        "red" => Rgba::rgb(255, 0, 0),
        "green" => Rgba::rgb(0, 128, 0),
        "blue" => Rgba::rgb(0, 0, 255),
        "gray" | "grey" => Rgba::rgb(128, 128, 128),
        "silver" => Rgba::rgb(192, 192, 192),
        "maroon" => Rgba::rgb(128, 0, 0),
        "yellow" => Rgba::rgb(255, 255, 0),
        "teal" => Rgba::rgb(0, 128, 128),
        "navy" => Rgba::rgb(0, 0, 128),
        _ => return None,
    };
    Some(color)
}

#[cfg(test)]
mod tests {
    use super::StyleEngine;
    use super::{parse_color, parse_font_size, parse_size};
    use crate::{SizeValue, parser};

    #[test]
    fn parses_hex_and_rgb_colors() {
        assert_eq!(parse_color("#fff").expect("color").r, 255);
        assert_eq!(parse_color("rgb(10, 20, 30)").expect("color").g, 20);
    }

    #[test]
    fn parses_font_size() {
        assert_eq!(parse_font_size("1.5em", 16.0), Some(24.0));
        assert_eq!(parse_font_size("small", 16.0), Some(13.0));
    }

    #[test]
    fn parses_size_variants() {
        assert_eq!(parse_size("100%", 16.0), Some(SizeValue::Percent(100.0)));
        assert_eq!(parse_size("300", 16.0), Some(SizeValue::Px(300.0)));
    }

    #[test]
    fn style_block_applies_to_elements() {
        let dom = parser::parse("<style>p { color: #ff0000; }</style><p>hello</p>");
        let mut engine = StyleEngine::default();
        let tree = engine.compute(&dom, false);

        fn find_first_tag<'a>(
            node: &'a crate::StyledNode,
            tag: &str,
        ) -> Option<&'a crate::StyledNode> {
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

        let p = find_first_tag(&tree, "p").expect("p");
        assert_eq!(p.style.color.r, 255);
    }
}
