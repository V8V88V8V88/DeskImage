mod gui;

fn main() {
    // Initialize logging
    env_logger::init();
    
    // Ensure application directories exist
    ensure_app_dirs();
    
    // Launch the GUI version
    if let Err(e) = gui::run_gui() {
        eprintln!("Error running GUI: {}", e);
    }
}

// Ensure application directories exist
fn ensure_app_dirs() -> bool {
    use std::fs;
    
    let home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("Could not determine home directory");
            return false;
        }
    };
    
    let local_bin = home_dir.join(".local/bin");
    let local_share = home_dir.join(".local/share");
    let local_apps = match dirs::data_dir() {
        Some(dir) => dir.join("applications"),
        None => home_dir.join(".local/share/applications"),
    };
    let local_icons = home_dir.join(".local/share/icons");
    
    println!("Local bin directory: {}", local_bin.display());
    println!("Local share directory: {}", local_share.display());
    println!("Applications directory: {}", local_apps.display());
    println!("Icons directory: {}", local_icons.display());
    
    // Create directories if they don't exist
    let directories = vec![
        ("bin", local_bin),
        ("share", local_share),
        ("applications", local_apps),
        ("icons", local_icons),
    ];
    
    for (name, path) in directories {
        if !path.exists() {
            println!("Creating {} directory: {}", name, path.display());
            if let Err(e) = fs::create_dir_all(&path) {
                eprintln!("Error creating {} directory: {}", name, e);
                continue;
            }
        }
    }
    
    // Check for environment variables
    if let Some(val) = std::env::var_os("XDG_DATA_HOME") {
        println!("XDG_DATA_HOME is set to: {:?}", val);
    } else {
        println!("XDG_DATA_HOME is not set");
    }
    
    true
}

// CLI implementation, now unused
#[allow(dead_code)]
fn run_cli() {
    use std::env;
    use std::fs;
    use std::io::{self};
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    let current_exe = env::current_exe().unwrap();
    let target_path = Path::new("/usr/local/bin/deskimage");

    if current_exe != target_path {
        println!("📦 DeskImage is not installed globally.");
        println!("⚙️  Do you want to install it to /usr/local/bin? [y/N]");

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();

        if choice.trim().to_lowercase() == "y" {
            let status = Command::new("sudo")
                .arg("cp")
                .arg(&current_exe)
                .arg(target_path)
                .status()
                .expect("Failed to execute sudo cp");

            if status.success() {
                println!("✅ Installed to /usr/local/bin. Now you can run `deskimage` globally.");
            } else {
                eprintln!("❌ Failed to install. Are you sure you have sudo?");
            }
            return;
        }
    }

    println!("🖼️  Enter path to your AppImage file:");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let appimage_path = PathBuf::from(input.trim());

    if !appimage_path.exists() {
        eprintln!("❌ File not found.");
        return;
    }

    let original_name = appimage_path.file_name().unwrap().to_string_lossy();
    let appname = clean_app_name(&original_name);

    let exec_target = dirs::home_dir().unwrap().join(".local/bin").join(&appname);

    fs::create_dir_all(exec_target.parent().unwrap()).unwrap();
    fs::copy(&appimage_path, &exec_target).unwrap();
    fs::set_permissions(&exec_target, fs::Permissions::from_mode(0o755)).unwrap();

    let desktop_content = format!(
        "[Desktop Entry]
Type=Application
Name={}
Exec={}
Icon=application-x-executable
Terminal=false
Categories=Utility;
",
        appname,
        exec_target.to_string_lossy()
    );

    let desktop_file_path = dirs::data_dir()
        .unwrap()
        .join("applications")
        .join(format!("{}.desktop", appname));
    fs::create_dir_all(desktop_file_path.parent().unwrap()).unwrap();
    fs::write(&desktop_file_path, desktop_content).unwrap();

    println!("✅ Desktop entry created at: {}", desktop_file_path.display());
}

fn clean_app_name(filename: &str) -> String {
    let base = filename
        .trim_end_matches(".AppImage")
        .split(|c: char| c == '-' || c == '_')
        .next()
        .unwrap_or(filename);
    base.to_string()
}
