use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DeskImageApp {
    appimage_path: Option<PathBuf>,
    status_message: String,
    is_installed: bool,
}

impl Default for DeskImageApp {
    fn default() -> Self {
        // Check if already installed globally
        let current_exe = std::env::current_exe().unwrap_or_default();
        let target_path = Path::new("/usr/local/bin/deskimage");
        
        // Check both if we're running from /usr/local/bin/deskimage
        // or if the file exists there (for when we're running from cargo or another location)
        let is_installed = current_exe == target_path || target_path.exists();

        Self {
            appimage_path: None,
            status_message: "Select an AppImage file to create a desktop entry".to_string(),
            is_installed,
        }
    }
}

impl DeskImageApp {
    fn install_globally(&mut self) {
        let current_exe = std::env::current_exe().unwrap_or_default();
        let target_path = Path::new("/usr/local/bin/deskimage");

        let status = Command::new("sudo")
            .arg("cp")
            .arg(&current_exe)
            .arg(target_path)
            .status();

        match status {
            Ok(status) if status.success() => {
                self.status_message = "✅ Installed to /usr/local/bin. Now you can run `deskimage` globally.".to_string();
                self.is_installed = true;
            }
            _ => {
                self.status_message = "❌ Failed to install. Are you sure you have sudo permissions?".to_string();
            }
        }
    }
    
    fn select_appimage(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("AppImage", &["AppImage"])
            .pick_file() {
            self.appimage_path = Some(path.clone());
            self.status_message = format!("Selected: {}", path.display());
            true
        } else {
            false
        }
    }
    
    // Parse a desktop entry file to extract key values
    fn parse_desktop_file(&self, content: &str) -> std::collections::HashMap<String, String> {
        let mut values = std::collections::HashMap::new();
        
        for line in content.lines() {
            if let Some(index) = line.find('=') {
                let key = line[..index].trim().to_string();
                let value = line[index+1..].trim().to_string();
                values.insert(key, value);
            }
        }
        
        values
    }
    
    fn create_desktop_entry(&mut self) {
        if let Some(appimage_path) = &self.appimage_path {
            if !appimage_path.exists() {
                self.status_message = "❌ File not found.".to_string();
                return;
            }

            let original_name = appimage_path.file_name().unwrap().to_string_lossy();
            let appname = self.clean_app_name(&original_name);

            if let Some(home_dir) = dirs::home_dir() {
                let exec_target = home_dir.join(".local/bin").join(&appname);
                
                // Create directory if it doesn't exist
                if let Err(e) = fs::create_dir_all(exec_target.parent().unwrap()) {
                    self.status_message = format!("❌ Couldn't create directory: {}", e);
                    return;
                }
                
                // Copy AppImage to target location
                if let Err(e) = fs::copy(appimage_path, &exec_target) {
                    self.status_message = format!("❌ Couldn't copy file: {}", e);
                    return;
                }
                
                // Set executable permissions
                if let Err(e) = fs::set_permissions(&exec_target, fs::Permissions::from_mode(0o755)) {
                    self.status_message = format!("❌ Couldn't set permissions: {}", e);
                    return;
                }

                if let Some(data_dir) = dirs::data_dir() {
                    let desktop_file_path = data_dir
                        .join("applications")
                        .join(format!("{}.desktop", appname));
                    
                    // Check if desktop entry already exists
                    let mut existing_icon = String::from("application-x-executable");
                    let mut existing_keywords = String::new();
                    let mut existing_categories = String::from("Utility;");
                    let mut existing_comment = String::new();
                    
                    if desktop_file_path.exists() {
                        if let Ok(content) = fs::read_to_string(&desktop_file_path) {
                            let values = self.parse_desktop_file(&content);
                            
                            // Preserve the custom icon if it exists
                            if let Some(icon) = values.get("Icon") {
                                existing_icon = icon.clone();
                            }
                            
                            // Preserve keywords
                            if let Some(keywords) = values.get("Keywords") {
                                existing_keywords = keywords.clone();
                            }
                            
                            // Preserve categories but ensure "Utility" is included
                            if let Some(categories) = values.get("Categories") {
                                if !categories.is_empty() {
                                    existing_categories = categories.clone();
                                    if !existing_categories.contains("Utility") {
                                        existing_categories = format!("Utility;{}", existing_categories);
                                    }
                                    // Ensure it ends with semicolon
                                    if !existing_categories.ends_with(';') {
                                        existing_categories.push(';');
                                    }
                                }
                            }
                            
                            // Preserve comment/description
                            if let Some(comment) = values.get("Comment") {
                                existing_comment = comment.clone();
                            }
                        }
                    }
                    
                    // Create desktop entry content with preserved values
                    let mut desktop_content = format!(
                        "[Desktop Entry]\nType=Application\nName={}\nExec={}\nIcon={}\nTerminal=false\n",
                        appname,
                        exec_target.to_string_lossy(),
                        existing_icon
                    );
                    
                    // Add optional fields if they exist
                    if !existing_categories.is_empty() {
                        desktop_content.push_str(&format!("Categories={}\n", existing_categories));
                    }
                    
                    if !existing_keywords.is_empty() {
                        desktop_content.push_str(&format!("Keywords={}\n", existing_keywords));
                    }
                    
                    if !existing_comment.is_empty() {
                        desktop_content.push_str(&format!("Comment={}\n", existing_comment));
                    }
                    
                    // Create directory if it doesn't exist
                    if let Err(e) = fs::create_dir_all(desktop_file_path.parent().unwrap()) {
                        self.status_message = format!("❌ Couldn't create applications directory: {}", e);
                        return;
                    }
                    
                    if let Err(e) = fs::write(&desktop_file_path, desktop_content) {
                        self.status_message = format!("❌ Couldn't write desktop file: {}", e);
                        return;
                    }

                    let message = if desktop_file_path.exists() {
                        format!("✅ Desktop entry updated at: {}", desktop_file_path.display())
                    } else {
                        format!("✅ Desktop entry created at: {}", desktop_file_path.display())
                    };
                    
                    self.status_message = message;
                } else {
                    self.status_message = "❌ Couldn't find data directory.".to_string();
                }
            } else {
                self.status_message = "❌ Couldn't find home directory.".to_string();
            }
        } else {
            self.status_message = "❌ No AppImage selected.".to_string();
        }
    }

    fn clean_app_name(&self, filename: &str) -> String {
        let base = filename
            .trim_end_matches(".AppImage")
            .split(|c: char| c == '-' || c == '_')
            .next()
            .unwrap_or(filename);
        base.to_string()
    }
}

impl eframe::App for DeskImageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure the UI style for a modern look
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 15.0);
        style.spacing.window_margin = Vec2::new(24.0, 24.0).into();
        
        // Dark theme
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = Color32::from_rgb(22, 22, 30);
        style.visuals.window_fill = Color32::from_rgb(22, 22, 30);
        style.visuals.faint_bg_color = Color32::from_rgb(35, 35, 45);
        style.visuals.extreme_bg_color = Color32::from_rgb(15, 15, 20);
        
        // Button styles
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(50, 50, 65);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 90);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(90, 90, 120);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 80, 100));
        
        // Apply the style
        ctx.set_style(style);
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Only show the header and installation section if not already installed
                if !self.is_installed {
                    ui.add_space(20.0);
                    
                    // Title with icon and styled text
                    ui.heading(RichText::new("🖼️ DeskImage").size(32.0).strong());
                    ui.add_space(5.0);
                    ui.label(RichText::new("Create desktop entries for AppImage files").size(16.0).color(Color32::from_rgb(180, 180, 200)));
                    
                    ui.add_space(30.0);
                    ui.separator();
                    ui.add_space(30.0);
                    
                    // Display installation section if not installed
                    ui.scope(|ui| {
                        ui.style_mut().visuals.extreme_bg_color = Color32::from_rgb(40, 30, 35);
                        egui::Frame::new()
                            .fill(Color32::from_rgb(40, 30, 35))
                            .corner_radius(12)
                            .stroke(Stroke::new(1.0, Color32::from_rgb(100, 60, 70)))
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("DeskImage is not installed globally").color(Color32::from_rgb(255, 150, 150)).size(16.0));
                                    ui.add_space(10.0);
                                    
                                    // Styled installation button
                                    let button = egui::Button::new(RichText::new("Install to /usr/local/bin").size(16.0).strong())
                                        .min_size(Vec2::new(250.0, 40.0))
                                        .fill(Color32::from_rgb(80, 50, 60));
                                    
                                    if ui.add(button).clicked() {
                                        self.install_globally();
                                    }
                                });
                            });
                    });
                    
                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);
                } else {
                    // A simpler header for the installed version
                    ui.add_space(20.0);
                    ui.heading(RichText::new("🖼️ DeskImage").size(32.0).strong());
                    ui.add_space(5.0);
                    ui.label(RichText::new("Create desktop entries for AppImage files").size(16.0).color(Color32::from_rgb(180, 180, 200)));
                    ui.add_space(20.0);
                }
                
                // File selection section with modern styling
                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 35, 45))
                    .corner_radius(12)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(60, 70, 100)))
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            // Styled file selection button
                            let select_button = egui::Button::new(RichText::new("Select AppImage File").size(16.0).strong())
                                .min_size(Vec2::new(250.0, 45.0))
                                .fill(Color32::from_rgb(60, 80, 120));
                            
                            if ui.add(select_button).clicked() {
                                self.select_appimage();
                            }
                            
                            ui.add_space(15.0);
                            
                            // Show selected file path with better styling
                            ui.label(RichText::new("Selected file:").size(14.0).color(Color32::from_rgb(170, 170, 190)));
                            
                            let path_text = if let Some(path) = &self.appimage_path {
                                path.display().to_string()
                            } else {
                                "No file selected".to_string()
                            };
                            
                            // Display the file path in a bordered frame
                            egui::Frame::new()
                                .fill(Color32::from_rgb(25, 25, 35))
                                .corner_radius(8)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(50, 50, 70)))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&path_text).monospace().size(14.0));
                                });
                            
                            ui.add_space(20.0);
                            
                            // Create desktop entry button with conditional styling
                            let create_button = egui::Button::new(
                                RichText::new("Create Desktop Entry").size(16.0).strong()
                            )
                            .min_size(Vec2::new(250.0, 45.0))
                            .fill(if self.appimage_path.is_some() {
                                Color32::from_rgb(60, 120, 80)
                            } else {
                                Color32::from_rgb(60, 60, 70)
                            });
                            
                            if ui.add_enabled(self.appimage_path.is_some(), create_button).clicked() {
                                self.create_desktop_entry();
                            }
                        });
                    });
                
                ui.add_space(25.0);
                
                // Status message with more visual separation and styling
                let (status_color, status_bg, status_border) = if self.status_message.starts_with("✅") {
                    (Color32::from_rgb(180, 255, 180), Color32::from_rgb(25, 45, 30), Color32::from_rgb(60, 120, 80))
                } else if self.status_message.starts_with("❌") {
                    (Color32::from_rgb(255, 180, 180), Color32::from_rgb(45, 25, 30), Color32::from_rgb(120, 60, 80))
                } else {
                    (Color32::from_rgb(220, 220, 220), Color32::from_rgb(35, 35, 45), Color32::from_rgb(70, 70, 90))
                };
                
                egui::Frame::new()
                    .fill(status_bg)
                    .corner_radius(10)
                    .stroke(Stroke::new(1.0, status_border))
                    .inner_margin(15.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(&self.status_message).size(14.0).color(status_color));
                    });
                
                ui.add_space(20.0);
                
                // Footer
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.label(RichText::new("© 2025 DeskImage").color(Color32::from_rgb(120, 120, 140)).size(12.0));
                    ui.add_space(5.0);
                });
            });
        });
    }
}

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 600.0])
            .with_min_inner_size([500.0, 400.0])
            .with_title("DeskImage")
            .with_decorations(true),
        ..Default::default()
    };
    
    eframe::run_native(
        "DeskImage",
        options,
        Box::new(|_cc| Ok(Box::new(DeskImageApp::default())))
    )
} 