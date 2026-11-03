use eframe::egui::{self, Color32, Pos2, Rect, Vec2};
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "background-image";

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    path: String,
    visible: bool,
    opacity: f32,
    scale: f32,
    position: [f32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_image_and_preserves_it_when_replacement_fails() {
        let path =
            std::env::temp_dir().join(format!("conway-background-{}.png", std::process::id()));
        image::RgbaImage::from_pixel(32, 16, image::Rgba([255, 0, 0, 128]))
            .save(&path)
            .unwrap();
        let ctx = egui::Context::default();
        let mut background = Background {
            path_input: path.to_string_lossy().into_owned(),
            ..Default::default()
        };
        background.load(&ctx, true);
        assert!(background.is_visible());
        assert_eq!(background.size, Vec2::new(32.0, 16.0));
        let loaded_path = background.settings.path.clone();
        background.path_input = path.join("missing.png").to_string_lossy().into_owned();
        background.load(&ctx, true);
        assert!(background.status.starts_with("Could not load image:"));
        assert_eq!(background.settings.path, loaded_path);
        assert!(background.is_visible());
        background.settings.visible = false;
        assert!(!background.is_visible());
        std::fs::remove_file(path).unwrap();
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: String::new(),
            visible: true,
            opacity: 0.5,
            scale: 1.0,
            position: [0.0; 2],
        }
    }
}

#[derive(Default)]
pub struct Background {
    settings: Settings,
    texture: Option<egui::TextureHandle>,
    size: Vec2,
    path_input: String,
    status: String,
    pub open: bool,
}

impl Background {
    pub fn restore(cc: &eframe::CreationContext<'_>) -> Self {
        let mut background = Self::default();
        if let Some(settings) = cc
            .storage
            .and_then(|s| eframe::get_value::<Settings>(s, STORAGE_KEY))
        {
            background.settings = settings;
            if !background.settings.opacity.is_finite() {
                background.settings.opacity = 0.5;
            }
            background.settings.opacity = background.settings.opacity.clamp(0.0, 1.0);
            if !background.settings.scale.is_finite() {
                background.settings.scale = 1.0;
            }
            background.settings.scale = background.settings.scale.clamp(0.01, 10.0);
            if !background.settings.position.iter().all(|v| v.is_finite()) {
                background.settings.position = [0.0; 2];
            }
            background.path_input = background.settings.path.clone();
            if !background.path_input.is_empty() {
                background.load(&cc.egui_ctx, false);
            }
        }
        background
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &self.settings);
    }

    fn load(&mut self, ctx: &egui::Context, reset: bool) {
        let result = (|| -> Result<_, String> {
            let path = std::path::Path::new(self.path_input.trim());
            let reader = image::ImageReader::open(path)
                .map_err(|e| e.to_string())?
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let image = reader.decode().map_err(|e| e.to_string())?;
            let size = Vec2::new(image.width() as f32, image.height() as f32);
            let limit = ctx.input(|i| i.max_texture_side).min(4096) as u32;
            let image = image.thumbnail(limit, limit).into_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            );
            let path = path.canonicalize().map_err(|e| e.to_string())?;
            Ok((path, size, pixels))
        })();
        match result {
            Ok((path, size, pixels)) => {
                self.texture = Some(ctx.load_texture(
                    "background-image",
                    pixels,
                    egui::TextureOptions::LINEAR,
                ));
                self.size = size;
                self.settings.path = path.to_string_lossy().into_owned();
                self.path_input = self.settings.path.clone();
                if reset {
                    self.settings.visible = true;
                    self.settings.position = [0.0; 2];
                    self.settings.scale = (800.0 / size.x).min(600.0 / size.y).min(1.0);
                }
                self.status = format!("Loaded: {} x {}", size.x, size.y);
            }
            Err(error) => {
                self.status = format!("Could not load image: {error}");
                self.open = true;
            }
        }
    }

    pub fn paint(&self, painter: &egui::Painter, origin: Pos2, zoom: f32) {
        if self.settings.visible
            && let Some(texture) = &self.texture
        {
            let min = origin + Vec2::from(self.settings.position) * zoom;
            painter.image(
                texture.id(),
                Rect::from_min_size(min, self.size * self.settings.scale * zoom),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::from_white_alpha((self.settings.opacity * 255.0).round() as u8),
            );
        }
    }

    pub fn is_visible(&self) -> bool {
        self.settings.visible && self.texture.is_some()
    }

    pub fn window(&mut self, ctx: &egui::Context) {
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.path_input = path.to_string_lossy().into_owned();
            self.load(ctx, true);
            self.open = true;
        }
        let mut open = self.open;
        egui::Window::new("Background image")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Image path (PNG, JPEG, WebP, GIF, BMP):");
                ui.add(egui::TextEdit::singleline(&mut self.path_input).desired_width(420.0));
                ui.small("You can also drop an image file onto the window.");
                if let Ok(cwd) = std::env::current_dir() {
                    ui.small(format!("Relative paths start at: {}", cwd.display()));
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.path_input.trim().is_empty(),
                            egui::Button::new("Load image"),
                        )
                        .clicked()
                    {
                        self.load(ctx, true);
                    }
                    if ui.button("Remove image").clicked() {
                        self.texture = None;
                        self.settings = Settings::default();
                        self.path_input.clear();
                        self.status.clear();
                    }
                });
                ui.add_enabled_ui(self.texture.is_some(), |ui| {
                    ui.checkbox(&mut self.settings.visible, "Show background");
                    ui.add(
                        egui::Slider::new(&mut self.settings.opacity, 0.0..=1.0).text("Opacity"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.settings.scale, 0.01..=10.0)
                            .logarithmic(true)
                            .text("Image scale"),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.add(egui::DragValue::new(&mut self.settings.position[0]).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut self.settings.position[1]).prefix("Y: "));
                    });
                });
                ui.label(&self.status);
                ui.small("Background is a tracing guide; CeTZ export contains only the graph.");
            });
        self.open = open;
    }
}
