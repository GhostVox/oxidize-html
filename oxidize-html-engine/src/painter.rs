use crate::{DrawCommand, LayoutNode, NodeContent, Point, Rect, TextAlign};

pub fn paint(node: &LayoutNode, commands: &mut Vec<DrawCommand>) {
    if let Some(bg) = node.style.background_color {
        commands.push(DrawCommand::FillRect {
            rect: node.rect,
            color: bg,
        });
    }

    paint_borders(node, commands);

    if let Some(origin) = node.bullet_origin {
        commands.push(DrawCommand::DrawText {
            text: "•".to_string(),
            origin,
            color: node.style.color,
            font_size: node.style.font_size,
        });
    }

    match &node.content {
        NodeContent::Box => {}
        NodeContent::Hr => {
            commands.push(DrawCommand::DrawLine {
                start: Point {
                    x: node.rect.x,
                    y: node.rect.y + node.rect.height / 2.0,
                },
                end: Point {
                    x: node.rect.right(),
                    y: node.rect.y + node.rect.height / 2.0,
                },
                color: node.style.border.top.color,
                width: node.style.border.top.width.max(1.0),
            });
        }
        NodeContent::Text(layout) => {
            for (index, line) in layout.lines.iter().enumerate() {
                let y = node.rect.y + (index as f32 * layout.line_height);
                let line_width = line.chars().count() as f32 * layout.font_size * 0.55;
                let x = match node.style.text_align {
                    TextAlign::Left => node.rect.x,
                    TextAlign::Center => node.rect.x + (node.rect.width - line_width) / 2.0,
                    TextAlign::Right => node.rect.x + node.rect.width - line_width,
                };
                commands.push(DrawCommand::DrawText {
                    text: line.clone(),
                    origin: Point { x, y },
                    color: node.style.color,
                    font_size: layout.font_size,
                });
            }
        }
        NodeContent::Image {
            width,
            height,
            source,
        } => {
            let rect = Rect {
                x: node.rect.x,
                y: node.rect.y,
                width: *width,
                height: *height,
            };
            if matches!(source, crate::image::ImageSource::Invalid) {
                commands.push(DrawCommand::DrawImagePlaceholder { rect });
            } else {
                commands.push(DrawCommand::DrawImage {
                    rect,
                    source: source.clone(),
                });
            }
        }
    }

    if let Some(href) = &node.style.href {
        commands.push(DrawCommand::Link {
            rect: node.rect,
            href: href.clone(),
        });
    }

    for child in &node.children {
        paint(child, commands);
    }
}

fn paint_borders(node: &LayoutNode, commands: &mut Vec<DrawCommand>) {
    let border = node.style.border;
    if border.top.width > 0.0 {
        commands.push(DrawCommand::DrawLine {
            start: Point {
                x: node.rect.x,
                y: node.rect.y,
            },
            end: Point {
                x: node.rect.right(),
                y: node.rect.y,
            },
            color: border.top.color,
            width: border.top.width,
        });
    }
    if border.right.width > 0.0 {
        commands.push(DrawCommand::DrawLine {
            start: Point {
                x: node.rect.right(),
                y: node.rect.y,
            },
            end: Point {
                x: node.rect.right(),
                y: node.rect.bottom(),
            },
            color: border.right.color,
            width: border.right.width,
        });
    }
    if border.bottom.width > 0.0 {
        commands.push(DrawCommand::DrawLine {
            start: Point {
                x: node.rect.x,
                y: node.rect.bottom(),
            },
            end: Point {
                x: node.rect.right(),
                y: node.rect.bottom(),
            },
            color: border.bottom.color,
            width: border.bottom.width,
        });
    }
    if border.left.width > 0.0 {
        commands.push(DrawCommand::DrawLine {
            start: Point {
                x: node.rect.x,
                y: node.rect.y,
            },
            end: Point {
                x: node.rect.x,
                y: node.rect.bottom(),
            },
            color: border.left.color,
            width: border.left.width,
        });
    }
}
