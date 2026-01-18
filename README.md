# RustGesture

A modern mouse gesture software for Windows written in Rust.

## Features

- **Mouse Gesture Recognition**: Track and recognize mouse gestures in 4 or 8 directions
- **Action Execution**: Execute keyboard shortcuts, mouse clicks, window commands, and run programs
- **Application-Specific Gestures**: Define different gestures for different applications
- **Gesture Modifiers**: Support for scroll wheel and additional mouse button modifiers
- **Configurable**: JSON-based configuration with global and app-specific gesture settings
- **Performance**: Low-latency gesture recognition using efficient algorithms

## Installation

### Prerequisites

- Windows 10 or later
- No additional dependencies required (statically linked)

### Build from Source

```bash
git clone https://github.com/yourusername/RustGesture.git
cd RustGesture
cargo build --release
```

The executable will be located at `target/release/rustgesture.exe`.

## Configuration

Configuration is stored in `%APPDATA%\RustGesture\config.json`.

### Default Gestures

The application comes with the following default gestures:

- **Right → Down**: Lock workstation (Win+L)
- **Down**: Minimize current window
- **Up**: Maximize current window

### Configuration Format

```json
{
  "global_gestures": {
    "Right → Down": {
      "Keyboard": {
        "keys": ["VK_LWIN", "L"]
      }
    },
    "Up": {
      "Window": {
        "command": "Maximize"
      }
    }
  },
  "app_gestures": {
    "chrome.exe": {
      "Left → Right": {
        "Mouse": {
          "button": "X1",
          "action_type": "Click"
        }
      }
    }
  },
  "disabled_apps": ["notepad.exe"],
  "settings": {
    "trigger_button": "Middle",
    "enable_8_direction": false,
    "initial_valid_move": 15,
    "effective_move": 20,
    "min_stroke_length": 30,
    "stay_timeout": 300
  }
}
```

### Action Types

#### Keyboard Action
```json
{
  "Keyboard": {
    "keys": ["VK_CONTROL", "VK_C"]
  }
}
```

#### Mouse Action
```json
{
  "Mouse": {
    "button": "Left|Right|Middle|X1|X2",
    "action_type": "Click|DoubleClick"
  }
}
```

#### Window Action
```json
{
  "Window": {
    "command": "Minimize|Maximize|Restore|Close|ShowDesktop"
  }
}
```

#### Run Action
```json
{
  "Run": {
    "command": "notepad.exe",
    "args": "C:\path\to\file.txt"
  }
}
```

### Gesture Direction Syntax

- **4 directions**: `Up`, `Down`, `Left`, `Right`
- **8 directions**: `Up`, `Down`, `Left`, `Right`, `UpLeft`, `UpRight`, `DownLeft`, `DownRight`
- **Multi-stroke gestures**: `Right → Down → Left`
- **With modifiers**: `Up + WheelForward`

### Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `trigger_button` | Mouse button to trigger gesture | `Middle` |
| `enable_8_direction` | Use 8-direction recognition for first stroke | `false` |
| `initial_valid_move` | Minimum movement (pixels) to start tracking | `15` |
| `effective_move` | Minimum movement (pixels) for new direction | `20` |
| `min_stroke_length` | Minimum stroke length (pixels) | `30` |
| `stay_timeout` | Timeout (ms) before gesture is cancelled | `300` |

## Usage

1. **Start the application**: Run `rustgesture.exe`
2. **Perform a gesture**:
   - Press and hold the trigger button (default: Middle Mouse Button)
   - Move mouse in the desired direction(s)
   - Release the trigger button
3. **Action executes automatically** when gesture is recognized

### Gesture Examples

- **Right → Down**: Lock workstation
- **Up**: Maximize window
- **Down**: Minimize window
- **Right**: Go forward (in browsers)
- **Left**: Go back (in browsers)

### Modifiers

You can use modifiers during gestures:

- **Scroll wheel forward/backward**: Adds `+ WheelForward` or `+ WheelBackward`
- **Additional mouse buttons**: Adds `+ LeftButton`, `+ RightButton`, or `+ MiddleButton`

Example: `Up + WheelForward` can be configured to perform a different action than just `Up`.

## Development

### Project Structure

```
RustGesture/
├── src/
│   ├── config/          # Configuration management
│   │   ├── config.rs    # Data structures
│   │   └── manager.rs   # Config loader/saver
│   ├── core/            # Gesture recognition engine
│   │   ├── gesture.rs   # Gesture data structures
│   │   ├── parser.rs    # Direction calculation
│   │   ├── tracker.rs   # Path tracking
│   │   ├── recognizer.rs # Gesture recognition
│   │   ├── intent.rs    # Intent matching
│   │   ├── executor.rs  # Action execution
│   │   └── app.rs       # Application integration
│   ├── winapi/          # Windows API bindings
│   │   ├── hook.rs      # Mouse hooks
│   │   ├── input.rs     # Input simulation
│   │   └── helpers.rs   # Utility functions
│   └── ui/              # User interface
│       └── tray.rs      # System tray
└── Cargo.toml
```

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

### Architecture

The application is organized into several layers:

1. **Input Layer** (`winapi/hook.rs`): Captures mouse events using Windows hooks
2. **Tracking Layer** (`core/tracker.rs`): Tracks mouse movement and detects gestures
3. **Recognition Layer** (`core/parser.rs`, `core/recognizer.rs`): Parses directions and recognizes gestures
4. **Matching Layer** (`core/intent.rs`): Matches gestures to configured actions
5. **Execution Layer** (`core/executor.rs`, `winapi/input.rs`): Executes actions
6. **Configuration Layer** (`config/`): Manages settings and gesture mappings

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Inspired by [WGestures](https://github.com/yingDev/WGestures)
- Built with [Rust](https://www.rust-lang.org/) and [windows-rs](https://github.com/microsoft/windows-rs)

## Roadmap

- [ ] Complete Windows message loop integration for system tray
- [ ] Add GUI settings editor
- [ ] Implement gesture recording feature
- [ ] Add more action types (clipboard manipulation, screenshots, etc.)
- [ ] Support for multiple monitors with different DPI
- [ ]Gesture visualization/trail effect
- [ ] Auto-update mechanism
