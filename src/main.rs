use std::os::unix::fs::PermissionsExt;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() {
    println!("🖼️  Enter path to your AppImage file:");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let appimage_path = PathBuf::from(input.trim());

    if !appimage_path.exists() {
        eprintln!("❌ File not found.");
        return;
    }

    let appname = appimage_path.file_stem().unwrap().to_string_lossy().to_string();
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
