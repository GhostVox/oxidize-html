use gpui::{Bounds, ParentElement, Pixels, Styled, div, img, px, rgb};
use oxidize_html_engine::{DrawCommand, Rect};
use std::path::PathBuf;
pub fn command_element(command: &DrawCommand) -> gpui::Div {
    match command {
        DrawCommand::FillRect { rect, color } => abs_rect(*rect).bg(to_gpui_color(*color)),
        DrawCommand::StrokeRect { rect, color, width } => {
            let mut layer = abs_rect(*rect);
            let w = width.max(1.0);
            let top = Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: w,
            };
            let right = Rect {
                x: rect.x + rect.width - w,
                y: rect.y,
                width: w,
                height: rect.height,
            };
            let bottom = Rect {
                x: rect.x,
                y: rect.y + rect.height - w,
                width: rect.width,
                height: w,
            };
            let left = Rect {
                x: rect.x,
                y: rect.y,
                width: w,
                height: rect.height,
            };
            for edge in [top, right, bottom, left] {
                layer = layer.child(abs_rect(edge).bg(to_gpui_color(*color)));
            }
            layer
        }
        DrawCommand::DrawText {
            text,
            origin,
            color,
            font_size,
        } => div()
            .absolute()
            .left(px(origin.x))
            .top(px(origin.y))
            .text_size(px(font_size.max(8.0)))
            .text_color(to_gpui_color(*color))
            .child(text.clone()),
        DrawCommand::DrawImagePlaceholder { rect } => abs_rect(*rect).bg(rgb(0xd1d5db)),
        DrawCommand::DrawImage { rect, source } => match source {
            oxidize_html_engine::image::ImageSource::LocalPath(path) => abs_rect(*rect).child(
                img(PathBuf::from(path))
                    .w(px(rect.width.max(1.0)))
                    .h(px(rect.height.max(1.0))),
            ),
            oxidize_html_engine::image::ImageSource::Remote(url) => abs_rect(*rect).child(
                img(url.clone())
                    .w(px(rect.width.max(1.0)))
                    .h(px(rect.height.max(1.0))),
            ),
            _ => abs_rect(*rect).bg(rgb(0xd1d5db)),
        },
        DrawCommand::DrawLine {
            start,
            end,
            color,
            width: _,
        } => {
            let dx = (end.x - start.x).abs();
            let dy = (end.y - start.y).abs();
            if dx >= dy {
                div()
                    .absolute()
                    .left(px(start.x.min(end.x)))
                    .top(px(start.y.min(end.y)))
                    .w(px(dx.max(1.0)))
                    .h(px(1.0))
                    .bg(to_gpui_color(*color))
            } else {
                div()
                    .absolute()
                    .left(px(start.x.min(end.x)))
                    .top(px(start.y.min(end.y)))
                    .w(px(1.0))
                    .h(px(dy.max(1.0)))
                    .bg(to_gpui_color(*color))
            }
        }
        DrawCommand::Link { rect, .. } => abs_rect(Rect {
            x: rect.x,
            y: rect.y + rect.height - 1.0,
            width: rect.width,
            height: 1.0,
        })
        .bg(rgb(0x0a66c2)),
    }
}

fn abs_rect(rect: Rect) -> gpui::Div {
    div()
        .absolute()
        .left(px(rect.x))
        .top(px(rect.y))
        .w(px(rect.width.max(0.0)))
        .h(px(rect.height.max(0.0)))
}

pub fn to_bounds_with_offset(rect: Rect, ox: f32, oy: f32) -> Bounds<Pixels> {
    gpui::bounds(
        gpui::point(px(rect.x + ox), px(rect.y + oy)),
        gpui::size(px(rect.width.max(0.0)), px(rect.height.max(0.0))),
    )
}

pub fn content_extent(commands: &[DrawCommand]) -> (f32, f32) {
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;

    for command in commands {
        match command {
            DrawCommand::FillRect { rect, .. }
            | DrawCommand::StrokeRect { rect, .. }
            | DrawCommand::DrawImagePlaceholder { rect }
            | DrawCommand::DrawImage { rect, .. }
            | DrawCommand::Link { rect, .. } => {
                max_x = max_x.max(rect.x + rect.width);
                max_y = max_y.max(rect.y + rect.height);
            }
            DrawCommand::DrawText {
                text,
                origin,
                font_size,
                ..
            } => {
                let w = (text.chars().count() as f32 * font_size * 0.55).max(*font_size);
                let h = font_size * 1.25;
                max_x = max_x.max(origin.x + w);
                max_y = max_y.max(origin.y + h);
            }
            DrawCommand::DrawLine {
                start, end, width, ..
            } => {
                max_x = max_x.max(start.x.max(end.x) + width);
                max_y = max_y.max(start.y.max(end.y) + width);
            }
        }
    }

    (max_x.max(960.0), max_y.max(720.0))
}

fn to_gpui_color(color: oxidize_html_engine::Rgba) -> gpui::Rgba {
    rgb(((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32)
}
