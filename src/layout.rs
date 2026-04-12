use crate::{
    Display, LayoutNode, NodeContent, Rect, SizeValue, StyledNode, TextLayout,
    image::{ImageSource, parse_source, source_dimensions},
};

#[derive(Default, Debug)]
pub struct LayoutEngine;

impl LayoutEngine {
    /// Takes a Root [`StyledNode`] and the available width for layout and computes the layout tree, returning the root [`LayoutNode`].
    pub fn compute(&mut self, root: &StyledNode, available_width: f32) -> LayoutNode {
        let (_, node) = layout_node(root, 0.0, 0.0, available_width);
        println!("Debug Layout:");
        println!();
        debug_layout_tree(&node, 0);
        node
    }
}

fn layout_node(node: &StyledNode, x: f32, y: f32, parent_width: f32) -> (f32, LayoutNode) {
    // Handle display: none nodes by creating a zero-sized box.
    if node.style.display == Display::None {
        let layout = LayoutNode {
            node_id: node.node_id,
            rect: Rect {
                x,
                y,
                width: 0.0,
                height: 0.0,
            },
            style: node.style.clone(),
            content: NodeContent::Box,
            bullet_origin: None,
            children: Vec::new(),
            tag: node.tag.clone(),
        };
        return (0.0, layout);
    }

    let margin = node.style.margin;
    let padding = node.style.padding;
    let content_x = x + margin.left + padding.left;
    let top = y + margin.top;
    let mut cursor_y = top + padding.top;

    let width = resolve_width(node.style.width, parent_width).max(0.0);
    let content_width = (width - padding.left - padding.right).max(0.0);

    let mut children = Vec::new();

    // TABLE ROW LOGIC
    if is_table_row(node) {
        let mut cursor_x = content_x;
        let mut row_height: f32 = 0.0;

        // Count non-empty/visible children to divide space fairly
        let visible_children: Vec<&StyledNode> = node
            .children
            .iter()
            .filter(|c| c.style.display != Display::None)
            .collect();
        let cell_count = visible_children.len();

        for (i, child) in visible_children.iter().enumerate() {
            let child_parent_width = match child.style.width {
                SizeValue::Px(px) => px.max(1.0),
                SizeValue::Percent(pct) => (content_width * (pct / 100.0)).max(1.0),
                SizeValue::Auto => {
                    // Robust Email Column Strategy:
                    if cell_count == 3 {
                        let weights = [0.65, 0.10, 0.25]; // Item / Qty / Price
                        (content_width * weights[i]).max(1.0)
                    } else if cell_count == 2 {
                        let weights = [0.75, 0.25]; // Label / Total
                        (content_width * weights[i]).max(1.0)
                    } else {
                        (content_width / cell_count as f32).max(1.0)
                    }
                }
            };

            let (height, child_layout) = layout_node(child, cursor_x, cursor_y, child_parent_width);
            cursor_x += child_layout.rect.width.max(0.0);
            row_height = row_height.max(height.max(child_layout.rect.height));
            children.push(child_layout);
        }

        // Apply row_height to all cells (Second pass)
        for child_layout in &mut children {
            child_layout.rect.height = row_height;
        }

        cursor_y += row_height;
    }
    // TABLE GROUP PASSTHROUGH (thead, tbody)
    else if matches!(node.tag.as_deref(), Some("thead" | "tbody" | "tfoot")) {
        for child in &node.children {
            let (height, child_layout) = layout_node(child, content_x, cursor_y, content_width);
            cursor_y += height;
            children.push(child_layout);
        }
    }
    else {

        // STANDARD BLOCK/INLINE LOGIC
        let line_start_x = content_x;
        let line_limit_x = line_start_x + content_width.max(1.0);
        let mut inline_cursor_x = line_start_x;
        let mut inline_line_height: f32 = 0.0;
        let mut in_inline_run = false;

        for child in &node.children {
            if is_inline_node(child) {
                in_inline_run = true;

                // FIXED <br> LOGIC:
                if child.tag.as_deref() == Some("br") {
                    // 1. End the current line and advance the cursor
                    cursor_y += inline_line_height.max(node.style.line_height);

                    // 2. RESET the line height so the NEXT line starts fresh
                    inline_line_height = 0.0;
                    inline_cursor_x = line_start_x;

                    // 3. Add the <br> to the children list so it shows in debug
                    let (_, _, br_layout) = layout_inline_node(
                        child,
                        line_start_x,
                        cursor_y - node.style.line_height,
                        1.0,
                    );
                    children.push(br_layout);
                    continue;
                }

                let (mut cw, mut ch, mut cl) = layout_inline_node(
                    child,
                    inline_cursor_x,
                    cursor_y,
                    (line_limit_x - line_start_x).max(1.0),
                );

                // Handle wrapping
                if inline_cursor_x > line_start_x && inline_cursor_x + cw > line_limit_x {
                    cursor_y += inline_line_height;
                    inline_cursor_x = line_start_x;
                    inline_line_height = 0.0;
                    let (nw, nh, nl) = layout_inline_node(
                        child,
                        inline_cursor_x,
                        cursor_y,
                        (line_limit_x - line_start_x).max(1.0),
                    );
                    cw = nw;
                    ch = nh;
                    cl = nl;
                }

                inline_cursor_x += cw;
                inline_line_height = inline_line_height.max(ch);
                children.push(cl);
            } else {
                if in_inline_run {
                    cursor_y += inline_line_height;
                    inline_cursor_x = line_start_x;
                    inline_line_height = 0.0;
                    in_inline_run = false;
                }

                let (height, child_layout) =
                    layout_node(child, content_x, cursor_y, content_width.max(1.0));
                if height > 0.0 || child_layout.tag.is_some() {
                    cursor_y += height;
                    children.push(child_layout);
                }
            }
        }
        if in_inline_run {
            cursor_y += inline_line_height;
        }
    }

    let mut own_content = NodeContent::Box;
    let mut intrinsic_height = 0.0;

    // ISSUE 1 FIX: Handling intrinsic height for Text/Image/Hr
    if let Some(text) = node.text.as_deref() {
        let layout = layout_text(
            text,
            node.style.font_size,
            node.style.line_height,
            content_width,
        );
        intrinsic_height = layout.lines.len() as f32 * layout.line_height;
        own_content = NodeContent::Text(layout);
    } else if node.tag.as_deref() == Some("img") {
        let source = node
            .attrs
            .get("src")
            .map(|s| parse_source(s))
            .unwrap_or(ImageSource::Invalid);
        let intrinsic = source_dimensions(&source).map(|(w, h)| (w as f32, h as f32));
        let (image_w, image_h) = resolve_image_size(
            node.style.width,
            node.style.height,
            content_width.max(1.0),
            intrinsic,
        );
        intrinsic_height = image_h;
        own_content = NodeContent::Image {
            source,
            width: image_w,
            height: image_h,
        };
    } else if node.tag.as_deref() == Some("hr") {
        intrinsic_height = 1.0;
        own_content = NodeContent::Hr;
    }

    let children_height = (cursor_y - (top + padding.top)).max(0.0);
    // Combine children heights with the node's own intrinsic height (text, etc)
    let content_height = children_height.max(intrinsic_height);

    let box_height = match node.style.height {
        SizeValue::Px(px) => px,
        _ => content_height + padding.top + padding.bottom,
    };

    let space_consumed = margin.top + box_height + margin.bottom;

    let rect = Rect {
        x: x + margin.left,
        y: top,
        width,
        height: box_height,
    };

    let out = LayoutNode {
        node_id: node.node_id,
        rect,
        style: node.style.clone(),
        content: own_content,
        bullet_origin: if node.style.display == Display::ListItem {
            Some(crate::Point {
                x: rect.x - 16.0,
                y: rect.y,
            })
        } else {
            None
        },
        children,
        tag: node.tag.clone(),
    };

    (space_consumed, out)
}

fn layout_inline_node(
    node: &StyledNode,
    x: f32,
    y: f32,
    line_max_width: f32,
) -> (f32, f32, LayoutNode) {
    let margin = node.style.margin;
    let padding = node.style.padding;
    let content_x = x + margin.left + padding.left;
    let content_y = y + margin.top + padding.top;
    let max_width = line_max_width.max(1.0);

    let mut own_content = NodeContent::Box;
    let mut intrinsic_width = 0.0;
    let mut intrinsic_height = 0.0;

    if let Some(text) = node.text.as_deref() {
        let layout = layout_text(
            text,
            node.style.font_size,
            node.style.line_height,
            max_width,
        );
        let char_width = (node.style.font_size * 0.55).max(1.0);
        intrinsic_width = layout
            .lines
            .iter()
            .map(|line| line.chars().count() as f32 * char_width)
            .fold(0.0, f32::max)
            .max(char_width);
        intrinsic_height = layout.lines.len() as f32 * layout.line_height;
        own_content = NodeContent::Text(layout);
    } else if node.tag.as_deref() == Some("img") {
        let source = node
            .attrs
            .get("src")
            .map(|s| parse_source(s))
            .unwrap_or(ImageSource::Invalid);
        let intrinsic = source_dimensions(&source).map(|(w, h)| (w as f32, h as f32));
        let (w, h) = resolve_image_size(node.style.width, node.style.height, max_width, intrinsic);
        intrinsic_width = w;
        intrinsic_height = h;
        own_content = NodeContent::Image {
            source,
            width: w,
            height: h,
        };
    } else if node.tag.as_deref() == Some("hr") {
        intrinsic_width = max_width;
        intrinsic_height = 1.0;
        own_content = NodeContent::Hr;
    } else if node.tag.as_deref() == Some("br") {
        // FIX: A <br> should occupy exactly one line of height
        intrinsic_height = node.style.line_height;
        own_content = NodeContent::Box;
    }

    let mut children = Vec::new();
    let mut child_x = content_x;
    let mut child_y = content_y;
    let line_start_x = content_x;
    let line_limit_x = line_start_x + max_width;
    let mut line_height = 0.0;
    let mut content_used_width = intrinsic_width;

    for child in &node.children {
        if is_inline_node(child) {
            let (mut cw, mut ch, mut cl) = layout_inline_node(child, child_x, child_y, max_width);
            if child_x > line_start_x && child_x + cw > line_limit_x {
                child_y += line_height;
                child_x = line_start_x;
                line_height = 0.0;
                let (nw, nh, nl) = layout_inline_node(child, child_x, child_y, max_width);
                cw = nw;
                ch = nh;
                cl = nl;
            }
            child_x += cw;
            line_height = line_height.max(ch);
            content_used_width = content_used_width.max(child_x - line_start_x);
            children.push(cl);
        } else {
            if line_height > 0.0 {
                child_y += line_height;
                child_x = line_start_x;
                line_height = 0.0;
            }
            let (bh, bl) = layout_node(child, line_start_x, child_y, max_width);
            child_y += bh;
            content_used_width = content_used_width.max(bl.rect.width);
            children.push(bl);
        }
    }
    if line_height > 0.0 {
        child_y += line_height;
    }

    let children_height = (child_y - content_y).max(0.0);
    let content_height = intrinsic_height.max(children_height);

    let resolved_width = match node.style.width {
        SizeValue::Px(px) => px,
        SizeValue::Percent(pct) => max_width * (pct / 100.0),
        SizeValue::Auto => content_used_width,
    };
    let width = resolved_width + padding.left + padding.right + margin.left + margin.right;
    let height = match node.style.height {
        SizeValue::Px(px) => px + margin.top + margin.bottom,
        _ => content_height + padding.top + padding.bottom + margin.top + margin.bottom,
    };

    let out = LayoutNode {
        node_id: node.node_id,
        rect: Rect {
            x: x + margin.left,
            y: y + margin.top,
            width: width.max(0.0),
            height: height.max(0.0),
        },
        style: node.style.clone(),
        content: own_content,
        bullet_origin: if node.style.display == Display::ListItem {
            Some(crate::Point {
                x: x + margin.left - 16.0,
                y: y + margin.top,
            })
        } else {
            None
        },
        children,
        tag: node.tag.clone(),
    };
    (out.rect.width, out.rect.height, out)
}

fn is_inline_display(display: Display) -> bool {
    matches!(display, Display::Inline | Display::InlineBlock)
}

fn is_inline_node(node: &StyledNode) -> bool {
    is_inline_display(node.style.display) || (node.text.is_some() && node.tag.is_none())
}

fn is_table_row(node: &StyledNode) -> bool {
    node.tag.as_deref() == Some("tr") || node.style.display == Display::TableRow
}

fn shift_layout_x(node: &mut LayoutNode, delta: f32) {
    node.rect.x += delta;
    if let Some(mut bullet) = node.bullet_origin {
        bullet.x += delta;
        node.bullet_origin = Some(bullet);
    }
    for child in &mut node.children {
        shift_layout_x(child, delta);
    }
}

fn resolve_width(size: SizeValue, parent_width: f32) -> f32 {
    match size {
        SizeValue::Px(px) => px,
        SizeValue::Percent(p) => parent_width * (p / 100.0),
        SizeValue::Auto => parent_width,
    }
}

fn resolve_image_size(
    width: SizeValue,
    height: SizeValue,
    max_width: f32,
    intrinsic: Option<(f32, f32)>,
) -> (f32, f32) {
    let explicit_w = match width {
        SizeValue::Px(px) => Some(px),
        SizeValue::Percent(p) => Some(max_width * (p / 100.0)),
        SizeValue::Auto => None,
    };
    let explicit_h = match height {
        SizeValue::Px(px) => Some(px),
        SizeValue::Percent(_) | SizeValue::Auto => None,
    };

    match (explicit_w, explicit_h, intrinsic) {
        (Some(w), Some(h), _) => (w.max(1.0), h.max(1.0)),
        (Some(w), None, Some((iw, ih))) if iw > 0.0 => (w.max(1.0), (w * ih / iw).max(1.0)),
        (None, Some(h), Some((iw, ih))) if ih > 0.0 => ((h * iw / ih).max(1.0), h.max(1.0)),
        (None, None, Some((iw, ih))) if iw > 0.0 => {
            let w = iw.min(max_width).max(1.0);
            let h = (w * ih / iw).max(1.0);
            (w, h)
        }
        (Some(w), None, None) => (w.max(1.0), 24.0),
        (None, Some(h), None) => (max_width.min(320.0).max(1.0), h.max(1.0)),
        (None, None, None) => (max_width.min(320.0).max(1.0), 180.0),
        _ => (max_width.min(320.0).max(1.0), 180.0),
    }
}

fn layout_text(text: &str, font_size: f32, line_height: f32, max_width: f32) -> TextLayout {
    let char_width = (font_size * 0.55).max(1.0);
    let max_chars = ((max_width / char_width).floor().max(1.0)) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    TextLayout {
        lines,
        line_height: if line_height > 0.0 {
            line_height
        } else {
            font_size * 1.2
        },
        font_size,
    }
}
fn debug_layout_tree(node: &LayoutNode, indent: usize) {
    let indent_str = "  ".repeat(indent);

    // 1. Differentiate between Tags and Text/Images
    let label = if let Some(tag) = &node.tag {
        format!("[<{}>]", tag)
    } else {
        match &node.content {
            crate::NodeContent::Text(layout) => {
                let text_snippet: String = layout.lines.join(" ").chars().take(20).collect();
                format!("\"{}...\"", text_snippet.escape_debug())
            }
            crate::NodeContent::Image { source, .. } => format!("[<img> {:?}]", source),
            crate::NodeContent::Hr => "[<hr>]".to_string(),
            crate::NodeContent::Box => "[<box>]".to_string(),
        }
    };

    // 2. Print Geometry in a readable format
    // Using green for geometry to make it pop against the labels
    print!(
        "{}{:<25} \x1b[32mpos:({:>4.1}, {:>4.1}) size:[{:>4.1} x {:>4.1}]\x1b[0m",
        indent_str, label, node.rect.x, node.rect.y, node.rect.width, node.rect.height
    );

    // 3. Add specific content indicators
    if let crate::NodeContent::Text(layout) = &node.content {
        print!(" \x1b[35m(lines: {})\x1b[0m", layout.lines.len());
    }

    println!(); // End line

    // 4. Recurse
    for child in &node.children {
        debug_layout_tree(child, indent + 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::{HtmlRenderer, LayoutNode, NodeContent};

    fn find_first_text(node: &LayoutNode) -> Option<&crate::TextLayout> {
        if let NodeContent::Text(layout) = &node.content {
            return Some(layout);
        }
        for child in &node.children {
            if let Some(found) = find_first_text(child) {
                return Some(found);
            }
        }
        None
    }

    fn collect_text_positions(node: &LayoutNode, out: &mut Vec<(String, f32, f32)>) {
        if let NodeContent::Text(layout) = &node.content {
            let text = layout.lines.join(" ");
            out.push((text, node.rect.x, node.rect.y));
        }
        for child in &node.children {
            collect_text_positions(child, out);
        }
    }

    #[test]
    fn wraps_text_into_multiple_lines() {
        let html = "<div style='width:120px'>This is a long line of text for wrapping</div>";
        let mut renderer = HtmlRenderer::default();
        let mut style = renderer.style_tree(html);
        crate::table::normalize_tables(&mut style, 120.0);
        let mut engine = super::LayoutEngine;
        let layout = engine.compute(&style, 120.0);
        let text = find_first_text(&layout).expect("text");
        assert!(text.lines.len() > 1);
    }

    #[test]
    fn inline_children_wrap_left_to_right() {
        let html = "<div style='width:120px'><span>aaaaaa</span><span>bbbbbb</span><span>cccccc</span></div>";
        let mut renderer = HtmlRenderer::default();
        let mut style = renderer.style_tree(html);
        crate::table::normalize_tables(&mut style, 120.0);
        let mut engine = super::LayoutEngine;
        let layout = engine.compute(&style, 120.0);
        let mut texts = Vec::new();
        collect_text_positions(&layout, &mut texts);

        let a = texts
            .iter()
            .find(|(t, _, _)| t.contains("aaaaaa"))
            .expect("text a");
        let b = texts
            .iter()
            .find(|(t, _, _)| t.contains("bbbbbb"))
            .expect("text b");
        let c = texts
            .iter()
            .find(|(t, _, _)| t.contains("cccccc"))
            .expect("text c");

        assert!(b.1 >= a.1 || b.2 > a.2);
        assert!(c.2 >= b.2);
    }

    #[test]
    fn table_row_cells_layout_horizontally() {
        let html = r#"<table width="600"><tr><td width="200">A</td><td width="300">B</td><td width="100">C</td></tr></table>"#;
        let mut renderer = HtmlRenderer::default();
        let mut style = renderer.style_tree(html);
        crate::table::normalize_tables(&mut style, 600.0);
        let mut engine = super::LayoutEngine;
        let layout = engine.compute(&style, 600.0);

        let mut cells = Vec::new();
        collect_cells(&layout, &mut cells);
        assert_eq!(cells.len(), 3);
        assert!(cells[1].rect.x > cells[0].rect.x);
        assert!(cells[2].rect.x > cells[1].rect.x);
        assert!((cells[1].rect.x - cells[0].rect.x - cells[0].rect.width).abs() < 1.0);
        assert!((cells[2].rect.x - cells[1].rect.x - cells[1].rect.width).abs() < 1.0);
    }

    #[test]
    fn colspan_cell_advances_row_cursor_by_resolved_width() {
        let html = r#"
            <table width="520">
              <tr>
                <td colspan="2" width="400" align="right">Subtotal</td>
                <td width="120" align="right">$249.96</td>
              </tr>
            </table>
        "#;
        let mut renderer = HtmlRenderer::default();
        let mut style = renderer.style_tree(html);
        crate::table::normalize_tables(&mut style, 520.0);
        let mut engine = super::LayoutEngine;
        let layout = engine.compute(&style, 520.0);

        let mut rows = Vec::new();
        collect_rows(&layout, &mut rows);
        let row = rows.into_iter().next().expect("row");
        let cells: Vec<&LayoutNode> = row
            .children
            .iter()
            .filter(|n| matches!(n.style.display, crate::Display::TableCell))
            .collect();
        assert_eq!(cells.len(), 2);
        assert!((cells[1].rect.x - (cells[0].rect.x + 400.0)).abs() < 1.0);
    }

    fn collect_rows<'a>(node: &'a LayoutNode, out: &mut Vec<&'a LayoutNode>) {
        if matches!(node.style.display, crate::Display::TableRow) {
            out.push(node);
            return;
        }
        for child in &node.children {
            collect_rows(child, out);
        }
    }

    fn collect_cells<'a>(node: &'a LayoutNode, out: &mut Vec<&'a LayoutNode>) {
        if matches!(node.style.display, crate::Display::TableCell)
            && matches!(node.content, NodeContent::Box)
        {
            out.push(node);
        }
        for child in &node.children {
            collect_cells(child, out);
        }
    }
}
