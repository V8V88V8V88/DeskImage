use eframe::egui;
use egui::{Color32, RichText, Stroke, Vec2};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DeskImageApp {
    appimage_path: Option<PathBuf>,
    icon_path: Option<PathBuf>,
    status_message: String,
    is_installed: bool,
    status_visible: bool,
    status_timestamp: std::time::Instant,
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
            icon_path: None,
            status_message: "Select an AppImage file to create a desktop entry".to_string(),
            is_installed,
            status_visible: true,
            status_timestamp: std::time::Instant::now(),
        }
    }
}

impl DeskImageApp {
    // Add a helper method to update status messages
    fn update_status(&mut self, message: String) {
        println!("Status update: {}", message);
        self.status_message = message;
        self.status_timestamp = std::time::Instant::now();
        self.status_visible = true;
    }

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
                self.update_status("SUCCESS: Installed to /usr/local/bin. Now you can run `deskimage` globally.".to_string());
                self.is_installed = true;
            }
            _ => {
                self.update_status("ERROR: Failed to install. Are you sure you have sudo permissions?".to_string());
            }
        }
    }
    
    fn select_appimage(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("AppImage", &["AppImage"])
            .pick_file() {
            
            // Make the AppImage executable when it's selected
            if !self.is_executable(&path) {
                println!("AppImage is not executable, setting executable permissions");
                
                if let Err(e) = self.make_executable(&path) {
                    println!("Warning: Couldn't set permissions on source AppImage: {}", e);
                    self.update_status(format!("WARNING: Couldn't make AppImage executable: {}", e));
                } else {
                    // Verify the AppImage is now executable
                    if self.is_executable(&path) {
                        println!("Successfully made AppImage executable: {}", path.display());
                    } else {
                        println!("Warning: AppImage may not be executable despite permissions change");
                        self.update_status(format!("WARNING: AppImage may not be executable despite permissions change"));
                    }
                }
            } else {
                println!("AppImage is already executable: {}", path.display());
            }
            
            self.appimage_path = Some(path.clone());
            self.update_status(format!("Selected: {}", path.display()));
            true
        } else {
            false
        }
    }
    
    fn select_icon(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Icons", &["png", "svg", "xpm", "jpg", "jpeg"])
            .pick_file() {
            self.icon_path = Some(path.clone());
            self.update_status(format!("Selected icon: {}", path.display()));
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
        println!("Creating desktop entry...");
        
        if let Some(appimage_path) = &self.appimage_path {
            if !appimage_path.exists() {
                println!("File not found: {}", appimage_path.display());
                self.update_status(format!("ERROR: File not found: {}", appimage_path.display()));
                return;
            }

            let original_name = match appimage_path.file_name() {
                Some(name) => name.to_string_lossy(),
                None => {
                    println!("Invalid file path: no filename");
                    self.update_status("ERROR: Invalid file path: no filename".to_string());
                    return;
                }
            };
            let appname = self.clean_app_name(&original_name);
            println!("App name: {}", appname);

            match dirs::home_dir() {
                Some(home_dir) => {
                    let exec_target = home_dir.join(".local/bin").join(&appname);
                    
                    // Create directory if it doesn't exist
                    match fs::create_dir_all(exec_target.parent().unwrap()) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't create directory: {}", e);
                            self.update_status(format!("ERROR: Couldn't create directory {}: {}", 
                                exec_target.parent().unwrap().display(), e));
                            return;
                        }
                    }
                    
                    // First, make sure the source AppImage is executable
                    if !self.is_executable(appimage_path) {
                        println!("Source AppImage is not executable, setting executable permissions");
                        if let Err(e) = self.make_executable(appimage_path) {
                            println!("Warning: Couldn't make source AppImage executable: {}", e);
                            // Continue anyway, we'll set permissions on the target
                        }
                    } else {
                        println!("Source AppImage is already executable");
                    }
                    
                    // Then copy it to the target location
                    match fs::copy(appimage_path, &exec_target) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't copy file: {}", e);
                            self.update_status(format!("ERROR: Couldn't copy file to {}: {}", 
                                exec_target.display(), e));
                            return;
                        }
                    }
                    
                    // Set executable permissions on the destination file
                    match self.make_executable(&exec_target) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't set permissions: {}", e);
                            self.update_status(format!("ERROR: Couldn't set permissions on {}: {}", 
                                exec_target.display(), e));
                            return;
                        }
                    }

                    // First try XDG_DATA_HOME, then fallback to ~/.local/share
                    let applications_dir = match dirs::data_dir() {
                        Some(dir) => dir.join("applications"),
                        None => home_dir.join(".local/share/applications"),
                    };
                    
                    println!("Applications directory: {}", applications_dir.display());
                    
                    // Ensure the applications directory exists
                    match fs::create_dir_all(&applications_dir) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't create applications directory: {}", e);
                            self.update_status(format!("ERROR: Couldn't create applications directory {}: {}", 
                                applications_dir.display(), e));
                            return;
                        }
                    }
                    
                    let desktop_file_path = applications_dir.join(format!("{}.desktop", appname));
                    println!("Desktop file path: {}", desktop_file_path.display());
                    
                    // Check if the desktop entry already exists before we start
                    let desktop_existed = desktop_file_path.exists();
                    println!("Desktop file existed before: {}", desktop_existed);
                    
                    // Check if desktop entry already exists
                    let mut existing_icon = String::from("application-x-executable");
                    let mut existing_keywords = String::new();
                    let mut existing_categories = String::from("Utility;");
                    let mut existing_comment = String::new();
                    
                    if desktop_file_path.exists() {
                        if let Ok(content) = fs::read_to_string(&desktop_file_path) {
                            let values = self.parse_desktop_file(&content);
                            
                            // Preserve the custom icon if it exists and no new one is selected
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
                    
                    // Handle custom icon if selected
                    let icon_value = if let Some(icon_path) = &self.icon_path {
                        // Copy the icon to the local icons directory if it exists
                        if icon_path.exists() {
                            let icon_filename = icon_path.file_name().unwrap().to_string_lossy();
                            let icon_path_string = icon_path.to_string_lossy().to_string();
                            let icon_destination = home_dir
                                .join(".local/share/icons")
                                .join(&*icon_filename);
                            
                            // Create icons directory if it doesn't exist
                            let icon_result = icon_path_string.clone();
                            if let Err(e) = fs::create_dir_all(icon_destination.parent().unwrap()) {
                                println!("Couldn't create icons directory: {}", e);
                                let warning = format!("WARNING: Couldn't create icons directory: {}", e);
                                self.update_status(warning);
                                // Continue with the original path as fallback
                                icon_result
                            } else {
                                // Copy the icon file
                                if let Err(e) = fs::copy(icon_path, &icon_destination) {
                                    println!("Couldn't copy icon: {}", e);
                                    let warning = format!("WARNING: Couldn't copy icon: {}", e);
                                    self.update_status(warning);
                                    // Continue with the original path as fallback
                                    icon_result
                                } else {
                                    // Use the icon destination path
                                    icon_destination.to_string_lossy().to_string()
                                }
                            }
                        } else {
                            // Icon doesn't exist, fall back to default
                            existing_icon
                        }
                    } else {
                        // No new icon selected, use existing
                        existing_icon
                    };
                    
                    // Create desktop entry content with preserved or new icon value
                    let mut desktop_content = format!(
                        "[Desktop Entry]\nType=Application\nName={}\nExec={}\nIcon={}\nTerminal=false\n",
                        appname,
                        exec_target.to_string_lossy(),
                        icon_value
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
                    
                    // Write the desktop file
                    match fs::write(&desktop_file_path, desktop_content) {
                        Ok(_) => {
                            println!("Successfully wrote desktop file");
                        },
                        Err(e) => {
                            println!("Couldn't write desktop file: {}", e);
                            self.update_status(format!("ERROR: Couldn't write desktop file {}: {}", 
                                desktop_file_path.display(), e));
                            return;
                        }
                    }
                    
                    // Attempt to update the desktop database to make it immediately visible
                    println!("Updating desktop database...");
                    match Command::new("update-desktop-database")
                        .arg(applications_dir.to_string_lossy().to_string())
                        .status() {
                        Ok(status) => println!("update-desktop-database exited with: {}", status),
                        Err(e) => println!("Failed to run update-desktop-database: {}", e),
                    };

                    // Update the icon cache using gtk-update-icon-cache if available
                    println!("Updating icon cache...");
                    match Command::new("gtk-update-icon-cache")
                        .arg("-f")
                        .arg("-t")
                        .arg(home_dir.join(".local/share/icons"))
                        .status() {
                        Ok(status) => println!("gtk-update-icon-cache exited with: {}", status),
                        Err(e) => println!("Failed to run gtk-update-icon-cache: {}", e),
                    };

                    // Verify the desktop entry was created successfully
                    match fs::metadata(&desktop_file_path) {
                        Ok(_) => {
                            println!("Successfully verified desktop entry exists");
                            let message = if desktop_existed {
                                format!("SUCCESS: Desktop entry updated at: {}", desktop_file_path.display())
                            } else {
                                format!("SUCCESS: Desktop entry created at: {}", desktop_file_path.display())
                            };
                            println!("Setting status message: {}", message);
                            self.update_status(message);
                        },
                        Err(e) => {
                            println!("Failed to verify desktop entry: {}", e);
                            self.update_status(format!("ERROR: Desktop entry may not have been created properly. Error: {}", e));
                        }
                    }
                },
                None => {
                    self.update_status("❌ Couldn't find home directory.".to_string());
                }
            }
        } else {
            self.update_status("❌ No AppImage selected.".to_string());
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

    // Helper function to check if a file is executable
    fn is_executable<P: AsRef<Path>>(&self, path: P) -> bool {
        if let Ok(metadata) = fs::metadata(&path) {
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            return mode & 0o111 != 0; // Check if any executable bit is set
        }
        false
    }

    // Helper function to make a file executable
    fn make_executable<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
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
        
        // Store current status to detect changes
        let previous_status = self.status_message.clone();
        
        // We need to keep updating the UI to animate status messages
        if self.status_visible {
            // Check if we need to repaint the UI
            const STATUS_DURATION: std::time::Duration = std::time::Duration::from_secs(10);
            let time_since_status = self.status_timestamp.elapsed();
            
            if time_since_status < STATUS_DURATION {
                // Request continuous repaints while the status is visible
                ctx.request_repaint();
            } else {
                // Keep status visible but stop continuous repaints after duration
                self.status_visible = false;
            }
        }
        
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

                            // Custom icon selection button
                            let icon_button = egui::Button::new(RichText::new("Select Custom Icon").size(16.0).strong())
                                .min_size(Vec2::new(250.0, 45.0))
                                .fill(Color32::from_rgb(60, 100, 100));
                            
                            if ui.add(icon_button).clicked() {
                                self.select_icon();
                            }
                            
                            ui.add_space(15.0);
                            
                            // Show selected icon path with styling
                            ui.label(RichText::new("Custom icon:").size(14.0).color(Color32::from_rgb(170, 170, 190)));
                            
                            let icon_text = if let Some(path) = &self.icon_path {
                                path.display().to_string()
                            } else {
                                "Default icon will be used".to_string()
                            };
                            
                            // Display the icon path in a bordered frame
                            egui::Frame::new()
                                .fill(Color32::from_rgb(25, 25, 35))
                                .corner_radius(8)
                                .stroke(Stroke::new(1.0, Color32::from_rgb(50, 50, 70)))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&icon_text).monospace().size(14.0));
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
                                println!("Create Desktop Entry button clicked");
                                
                                // Change the status message immediately to show we're processing
                                self.update_status("Processing...".to_string());
                                
                                // Then create the desktop entry
                                self.create_desktop_entry();
                            }
                        });
                    });
                
                ui.add_space(25.0);
                
                // Status message with more visual separation and styling
                let (status_color, status_bg, status_border) = if self.status_message.starts_with("SUCCESS") {
                    (Color32::from_rgb(180, 255, 180), Color32::from_rgb(25, 45, 30), Color32::from_rgb(60, 120, 80))
                } else if self.status_message.starts_with("ERROR") {
                    (Color32::from_rgb(255, 180, 180), Color32::from_rgb(45, 25, 30), Color32::from_rgb(120, 60, 80))
                } else if self.status_message.starts_with("WARNING") {
                    (Color32::from_rgb(255, 220, 150), Color32::from_rgb(45, 35, 20), Color32::from_rgb(120, 90, 40))
                } else {
                    (Color32::from_rgb(220, 220, 220), Color32::from_rgb(35, 35, 45), Color32::from_rgb(70, 70, 90))
                };
                
                // Create pulsing effect for new status messages
                let border_width = if self.status_visible {
                    // Calculate a pulsing border width between 1.0 and 3.0
                    let time_since_status = self.status_timestamp.elapsed().as_secs_f32();
                    let pulse = (time_since_status * 3.0).sin() * 0.5 + 0.5; // oscillate between 0.0 and 1.0
                    1.0 + pulse * 2.0 // between 1.0 and 3.0
                } else {
                    1.0 // default border width
                };
                
                // Debug text to show in UI
                let debug_text = format!(
                    "Status Message: {}\nStatus age: {:.1}s\nVisible: {}", 
                    self.status_message,
                    self.status_timestamp.elapsed().as_secs_f32(),
                    self.status_visible
                );
                
                egui::Frame::new()
                    .fill(status_bg)
                    .corner_radius(10)
                    .stroke(Stroke::new(border_width, status_border)) // Make border pulse
                    .inner_margin(20.0) // Increase margin
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.heading(RichText::new(&self.status_message).size(16.0).color(status_color).strong());
                            
                            // Display debug info in smaller text
                            ui.add_space(10.0);
                            ui.label(RichText::new(&debug_text).size(12.0).color(Color32::from_rgb(180, 180, 180)));
                        });
                    });
                
                ui.add_space(20.0);
                
                // Footer
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.label(RichText::new("© 2025 DeskImage").color(Color32::from_rgb(120, 120, 140)).size(12.0));
                    ui.add_space(5.0);
                });
            });
        });
        
        // If status message changed, update the timestamp and visibility
        if previous_status != self.status_message {
            println!("Status message changed: {}", self.status_message);
            self.status_timestamp = std::time::Instant::now();
            self.status_visible = true;
            ctx.request_repaint();
        }
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