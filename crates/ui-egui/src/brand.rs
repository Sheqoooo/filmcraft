//! Plain-text identification for this independent developer fork.
//! The upstream ArtCraft trademark images must not be embedded in modified builds.

/// Draw the fork name in the space previously occupied by the upstream wordmark.
pub fn paint_wordmark(ui: &egui::Ui, left_center: egui::Pos2, height: f32, _dark: bool) -> Option<egui::Rect> {
    Some(ui.painter().text(left_center, egui::Align2::LEFT_CENTER, "FilmCraft Dev", egui::FontId::proportional(height), ui.visuals().text_color()))
}
