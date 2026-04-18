use gpui::{
    App, Bounds, Context, DispatchPhase, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, MouseUpEvent, Pixels, Render, Window, WindowBounds, WindowOptions, div,
    img, prelude::*, px, rgb, size,
};
use oxidize_html::{DrawCommand, HtmlRenderer, Rect};
use oxidize_html_gpui::gpui_renderer::{command_element, content_extent, to_bounds_with_offset};
use std::{env, fs, path::PathBuf, sync::Arc};

const DEFAULT_HTML: &str = r##"
<table width="720" style="border:1px solid #dddddd; background-color:#fafafa;">
  <tr>
    <td width="240" style="padding:16px; background-color:#f0f7ff;">
      <h2 style="margin:0 0 8px 0;">Hello from oxidize-html_engine</h2>
      <p style="margin:0; color:#333333;">This is a GPUI preview of your HTML renderer.</p>
    </td>
    <td style="padding:16px;">
      <p style="font-size:14px; line-height:20px; color:#222;">Resize width in code, pass your own HTML file path, and inspect commands on the right panel.</p>
      <a href="https://example.com" style="color:#0a66c2;">example.com</a>
      <hr />
      <font color="#bb0000" size="4">Legacy font tag fallback is active.</font>
    </td>
  </tr>
</table>
"##;

struct DemoApp {
    html_label: String,
    html: String,
    render_width: f32,
    renderer: HtmlRenderer,
}

struct EmailView {
    commands: Vec<DrawCommand>,
    on_link_click: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

struct EmailViewLayoutState {
    element: gpui::AnyElement,
    commands: Vec<DrawCommand>,
}

impl Element for EmailView {
    type RequestLayoutState = EmailViewLayoutState;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some("email-view".into())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let commands = self.commands.clone();
        let (doc_width, doc_height) = content_extent(&commands);

        let mut document = div()
            .relative()
            .w(px(doc_width))
            .h(px(doc_height))
            .bg(rgb(0xffffff));
        for command in &commands {
            document = document.child(command_element(command));
        }
        let mut element = document.into_any_element();
        let layout_id = element.request_layout(window, cx);
        (layout_id, EmailViewLayoutState { element, commands })
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.element.paint(window, cx);

        let Some(callback) = self.on_link_click.as_ref().cloned() else {
            return;
        };

        let ox = f32::from(bounds.origin.x);
        let oy = f32::from(bounds.origin.y);
        for command in &request_layout.commands {
            if let DrawCommand::Link { rect, href } = command {
                let link_bounds = to_bounds_with_offset(*rect, ox, oy);
                let href = href.clone();
                let callback = callback.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, _cx| {
                    if phase == DispatchPhase::Bubble && link_bounds.contains(&event.position) {
                        callback(&href);
                    }
                });
            }
        }
    }
}

impl IntoElement for EmailView {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Render for DemoApp {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let window_width = f32::from(window.bounds().size.width);
        let target_width = (window_width - 360.0).max(240.0);
        if (target_width - self.render_width).abs() > f32::EPSILON {
            self.render_width = target_width;
        }
        let commands = self.renderer.render_html(&self.html, target_width, true);

        let lines_panel = div()
            .id("renderer-command-panel")
            .w(px(360.0))
            .h_full()
            .overflow_y_scroll()
            .bg(rgb(0x111827))
            .text_color(rgb(0xe5e7eb))
            .p_3()
            .gap_1()
            .flex()
            .flex_col()
            .child(format!("Source: {}", self.html_label))
            .child(format!("Render width: {:.0}px", self.render_width));

        let viewport = div()
            .id("renderer-scroll-viewport")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .overflow_x_scroll()
            .bg(rgb(0xffffff))
            .child(EmailView {
                commands,
                on_link_click: Some(Arc::new(|href: &str| {
                    println!("link clicked: {href}");
                })),
            });

        div()
            .size_full()
            .flex()
            .bg(rgb(0x0f172a))
            .child(viewport)
            .child(lines_panel)
    }
}

fn load_html() -> (String, String) {
    let arg = env::args().nth(1);
    if let Some(path) = arg {
        let path_buf = PathBuf::from(path);
        let label = path_buf.display().to_string();
        let html = fs::read_to_string(&path_buf).unwrap_or_else(|_| DEFAULT_HTML.to_string());
        return (label, html);
    }
    ("inline demo".to_string(), DEFAULT_HTML.to_string())
}

fn main() {
    let (label, html) = load_html();

    gpui::Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(860.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| DemoApp {
                    html_label: label.clone(),
                    html: html.clone(),
                    render_width: (f32::from(bounds.size.width) - 360.0).max(240.0),
                    renderer: HtmlRenderer::default(),
                })
            },
        )
        .unwrap();
    });
}
