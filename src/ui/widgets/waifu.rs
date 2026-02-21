use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui_image::{Resize, StatefulImage};

use crate::app::App;

pub fn draw_waifu(frame: &mut Frame, area: Rect, app: &mut App) {
    let protocol_name = format!("{:?}", app.picker.protocol_type());
    let category = app.cfg.waifu_category();
    let fetch_indicator = if app.waifu_fetching { " ..." } else { "" };

    let gallery_info = if !app.waifu_gallery.is_empty() && app.waifu_index >= 0 {
        format!(" [{}/{}]", app.waifu_index + 1, app.waifu_gallery.len())
    } else {
        String::new()
    };

    let title =
        format!(" Waifu [{protocol_name}] [{category}]{gallery_info} Live{fetch_indicator} ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(Color::Magenta));

    match &mut app.waifu_state {
        Some(state) => {
            let inner = block.inner(area);
            frame.render_widget(block, area);

            if inner.width > 0 && inner.height > 0 {
                // Crop fills the entire widget area (clips overflow edges).
                let image_widget = StatefulImage::new(None).resize(Resize::Crop(None));
                frame.render_stateful_widget(image_widget, inner, state);

                // Info overlay: show formatted name on bottom line.
                if app.waifu_show_info && !app.waifu_name.is_empty() && inner.height > 1 {
                    let overlay_area =
                        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
                    let overlay = Paragraph::new(app.waifu_name.as_str()).style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Black)
                            .add_modifier(Modifier::DIM),
                    );
                    frame.render_widget(overlay, overlay_area);
                }
            }
        }
        None => {
            let msg = if app.cfg.waifu_endpoint().is_some() {
                "Press 'f' to fetch from live service"
            } else {
                "No waifu endpoint configured"
            };
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }
}
