use ratatui::{
    layout::{Constraint, Rect},
    style::{Style, Stylize},
    widgets::Paragraph,
};

use crate::ui::utils::*;
use ratatui_image::{Resize, StatefulImage};

pub fn render_image_widget(
    frame: &mut ratatui::Frame<'_>,
    protocol: &mut crate::ui::Protocol,
    image_area: Rect,
) {
    use image::imageops::FilterType;

    if let Some(ref mut img) = protocol.image {
        // Get the image dimensions after resizing for the available area
        let resize = Resize::Scale(Some(FilterType::Lanczos3));
        let img_rect = img.size_for(resize.clone(), image_area);

        // Center the image within the available area
        let centered_area = center_image(img_rect, image_area);

        let image = StatefulImage::default().resize(resize);
        frame.render_stateful_widget(image, centered_area, img);
    } else {
        let placeholder_area =
            center_area(image_area, Constraint::Length(12), Constraint::Length(1));
        let placeholder = Paragraph::new("No album art").style(Style::default().dark_gray());
        frame.render_widget(placeholder, placeholder_area);
    }
}
