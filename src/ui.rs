use gpui::{
    div, prelude::*, px, svg, Div, FontWeight, Hsla, InteractiveElement, MouseButton, SharedString,
};

pub const FONT_FAMILY: &str = "Museo Sans";

pub const WEBSITE_URL: &str = "https://gs.femboy.tw";
pub const DISCORD_URL: &str = "https://discord.gg/YzDpcsEhCE";
pub const API_DOCS_URL: &str = "https://docs-csgo.neverlose.cc/";

pub fn plain(
    value: impl Into<SharedString>,
    size: f32,
    weight: f32,
    color: Hsla,
) -> Div {
    div()
        .font_family(FONT_FAMILY)
        .text_size(px(size))
        .font_weight(FontWeight(weight))
        .text_color(color)
        .child(value.into())
}

pub fn icon(path: &'static str, size: f32, color: Hsla) -> gpui::Svg {
    svg()
        .path(path)
        .flex_none()
        .size(px(size))
        .text_color(color)
}

pub fn no_drag_div<E: InteractiveElement>(element: E) -> E {
    element.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

#[derive(Debug, Clone)]
pub enum ChangelogSegment {
    Text(String),
    Link { label: String, url: String },
}

pub fn parse_changelog_line(line: &str) -> Vec<ChangelogSegment> {
    let mut segments = Vec::new();
    let mut rest = line;

    while let Some(start) = rest.find('[') {
        let Some(mid) = rest[start..].find("](") else {
            break;
        };
        let label = &rest[start + 1..start + mid];
        let after = &rest[start + mid + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let url = &after[..end];
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            break;
        }

        if start > 0 {
            segments.push(ChangelogSegment::Text(rest[..start].to_string()));
        }
        segments.push(ChangelogSegment::Link {
            label: label.to_string(),
            url: url.to_string(),
        });
        rest = &after[end + 1..];
    }

    if !rest.is_empty() {
        segments.push(ChangelogSegment::Text(rest.to_string()));
    }

    if segments.is_empty() {
        segments.push(ChangelogSegment::Text(line.to_string()));
    }

    segments
}

pub fn parse_changelog(value: Option<&str>) -> Vec<String> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return vec!["No changelog provided.".to_string()];
    };

    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            if let Some(stripped) = line.strip_prefix("- ") {
                format!("- {stripped}")
            } else if let Some(stripped) = line.strip_prefix("* ") {
                format!("- {stripped}")
            } else {
                line.to_string()
            }
        })
        .collect()
}
