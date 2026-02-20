use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui_image::StatefulImage;

use crate::app::App;

pub fn draw_waifu(frame: &mut Frame, area: Rect, app: &mut App) {
    let protocol_name = format!("{:?}", app.picker.protocol_type());
    let title = format!(" Waifu [{protocol_name}] ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(Color::Magenta));

    match &mut app.waifu_state {
        Some(state) => {
            // Render the border block first, then the image inside the inner area.
            let inner = block.inner(area);
            frame.render_widget(block, area);

            if inner.width > 0 && inner.height > 0 {
                let image_widget = StatefulImage::new(None);
                frame.render_stateful_widget(image_widget, inner, state);
            }
        }
        None => {
            let paragraph = Paragraph::new("No waifu cached")
                .style(Style::default().fg(Color::DarkGray))
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }
}
