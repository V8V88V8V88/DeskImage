# DeskImage

A modern GUI application to easily create desktop entries for AppImage files on Linux. Simplify your AppImage management with a sleek, intuitive interface.

![DeskImage Screenshot](screenshot.png)

## Features

- Modern egui-based user interface with dark theme
- Intuitive file selection for AppImage files
- Automatic app name extraction from filenames
- Proper desktop entry creation in the standard locations
- Global installation option for system-wide access
- Clean, responsive design adhering to 2025 UI standards

## Technical Details

- Built with Rust for performance and reliability
- Uses egui/eframe v0.31.1 with the latest API (compatible with breaking changes)
- File dialogs powered by rfd 0.12.1
- Dark mode with custom styling and visual elements
- Minimal dependencies for a lightweight experience

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/v8v88v8v88/DeskImage.git
cd DeskImage

# Build the project
cargo build --release

# Run the application
./target/release/deskimage
```

### Running the Application

Once built, you can run the application directly:

```bash
./target/release/deskimage
```

Or install it globally for easier access:

```bash
sudo cp ./target/release/deskimage /usr/local/bin/
```

## Usage

1. Launch the application from your terminal or application menu
2. If not installed globally, you can click the "Install to /usr/local/bin" button
3. Click "Select AppImage File" and browse to choose your AppImage file
4. Click "Create Desktop Entry" to generate the desktop entry

The application will:
- Copy the AppImage to `~/.local/bin/` with executable permissions
- Create a desktop entry file in `~/.local/share/applications/`
- Display success or failure status messages

## Requirements

- Linux-based operating system
- Rust 1.76.0 or later recommended (required for egui 0.31.1)
- Standard desktop environment (GNOME, KDE, XFCE, etc.)

## Development

Contributions are welcome! If you'd like to contribute:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### API Notes

DeskImage uses egui 0.31.1, which has several API changes compared to earlier versions:
- `Rounding` has been renamed to `CornerRadius`
- `Frame::none()` is replaced with `Frame::new()`
- `rounding()` method is renamed to `corner_radius()`
- Floating point values must be converted to appropriate types with `.into()`

## License

This project is licensed under the GNU General Public License v2.0 - see the [LICENSE](LICENSE) file for details.

## Author

- **v8v88v8v88** - *Initial work*

## Acknowledgments

- The Rust community for the amazing language and tools
- egui/eframe authors for the excellent UI framework
