use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui_image::StatefulImage;

use crate::app::App;

pub fn draw_waifu(frame: &mut Frame, area: Rect, app: &mut App) {
    let protocol_name = format!("{:?}", app.picker.protocol_type());
    let live = if app.cfg.waifu_endpoint().is_some() {
        " Live"
    } else {
        ""
    };
    let fetch_indicator = if app.waifu_fetching { " ..." } else { "" };
    let title = if !app.waifu_images.is_empty() && app.waifu_index >= 0 {
        format!(
            " Waifu [{protocol_name}] [{}/{}]{live}{fetch_indicator} ",
            app.waifu_index + 1,
            app.waifu_images.len()
        )
    } else {
        format!(" Waifu [{protocol_name}]{live}{fetch_indicator} ")
    };

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
                let image_widget = StatefulImage::new(None);
                frame.render_stateful_widget(image_widget, inner, state);

                // Info overlay: show formatted filename on bottom line.
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
                "No waifu cached"
            };
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }
}
