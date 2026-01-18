# Design: Mouse Gesture Software

## Overview
This document describes the architectural design for implementing a mouse gesture software in Rust, inspired by WGestures but leveraging Rust's strengths for better performance and safety.

## Architecture Principles
1. **Layered Architecture** - Clear separation between OS interaction, gesture recognition, and application logic
2. **Thread Safety** - Use message passing between threads (hook thread → processing thread → UI thread)
3. **Zero-Cost Abstractions** - Minimize runtime overhead in gesture recognition path
4. **Error Resilience** - Graceful degradation when hooks fail or configuration is invalid

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     UI Layer                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ System Tray  │  │ Settings UI  │  │ Gesture View │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                   Application Layer                          │
│  ┌──────────────────┐  ┌──────────────────────────────────┐ │
│  │ Gesture Parser   │  │  Configuration Manager            │ │
│  │  - Gesture State │  │  - Load/Save Config              │ │
│  │  - Direction Map │  │  - App-specific Rules            │ │
│  └────────┬─────────┘  └──────────────────────────────────┘ │
│           │                                                   │
│  ┌────────▼──────────────────────────────────────────────┐  │
│  │        Gesture Intent Finder                          │  │
│  │  - Match gesture to action                            │  │
│  │  - Context-aware (per-application)                    │  │
│  └────────┬──────────────────────────────────────────────┘  │
│           │                                                   │
│  ┌────────▼──────────────────────────────────────────────┐  │
│  │        Command Executor                               │  │
│  │  - Keyboard Simulation                                │  │
│  │  - Mouse Simulation                                   │  │
│  │  - Window Commands                                    │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                  Core Gesture Engine                         │
│  ┌────────────────────────────────────────────────────────┐ │
│  │         Path Tracker (Win32MousePathTracker)           │ │
│  │  - Mouse Hook Thread                                   │ │
│  │  - Low-level Input Capture                             │ │
│  │  - Movement Filtering & Validation                     │ │
│  └────────┬───────────────────────────────────────────────┘ │
│           │                                                   │
│  ┌────────▼──────────────────────────────────────────────┐  │
│  │       Gesture Context Detector                         │  │
│  │  - Detect active application                           │  │
│  │  - Fullscreen detection                                │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    OS Abstraction Layer                      │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │ Input Hook API   │  │  Input Sim API   │               │
│  │ - SetWindowsHook │  │ - SendInput      │               │
│  │ - LowLevelMouse  │  │ - keybd_event    │               │
│  │ - LowLevelKybd   │  │ - mouse_event    │               │
│  └──────────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Path Tracker (Core Input Capture)

**Responsibility**: Capture low-level mouse events and determine when to start tracking gestures.

**Key Design Decisions**:
- Run Windows hook in dedicated thread to avoid blocking UI
- Use message queue (similar to WGestures) to serialize events
- Implement "capture decision" - determine if gesture should start based on:
  - Current application (disabled list)
  - Fullscreen state
  - Mouse button used (Right, Middle, X1, X2)

**Critical Thresholds**:
- `initial_valid_move`: Minimum distance (5px) before gesture starts
- `effective_move`: Minimum distance (20px) to count as gesture direction
- `stay_timeout`: Cancel gesture if no movement for 500ms

**Thread Safety**:
```rust
// Hook thread captures events and sends to processing thread
hook_thread_event -> mpsc::channel -> processing_thread
```

### 2. Gesture Parser

**Responsibility**: Convert raw mouse path into gesture directions.

**Algorithm** (4-direction):
1. Calculate vector from last point to current point
2. Determine quadrant based on (dx, dy)
3. Within quadrant, compare |dx| vs |dy| to determine primary axis
4. Only add new direction if it differs from last direction

**Algorithm** (8-direction):
1. Calculate angle of movement vector
2. Divide 360° into 8 sectors (45° each)
3. Use fuzzy matching (50% slash range) to handle diagonal ambiguity
4. First stroke can be diagonal, subsequent strokes are 4-direction

**State Machine**:
```
Inactive → Tracking → Recognized → Executed
    ↑          ↓
    └────── Canceled
```

### 3. Gesture Intent Finder

**Responsibility**: Match recognized gestures to actions based on context.

**Matching Priority**:
1. Application-specific gesture → Global gesture
2. Gesture with modifiers → Gesture without modifiers
3. Longer gesture → Shorter gesture (prefix match)

**Configuration Structure**:
```rust
struct GestureConfig {
    global: HashMap<Gesture, Action>,
    app_specific: HashMap<AppId, HashMap<Gesture, Action>>,
    disabled_apps: HashSet<AppId>,
}
```

### 4. Input Simulator

**Responsibility**: Execute actions by simulating keyboard/mouse input.

**Design**:
- Use Windows `SendInput` API
- Tag simulated events with extra info to prevent re-capture
- Support:
  - Single key press
  - Key combinations (Ctrl+C, Alt+Tab)
  - Mouse movements and clicks
  - Window commands (minimize, maximize, close)

**Safety**:
- Always set `dwExtraInfo` flag on simulated events
- Filter out events with this flag in hook callback

### 5. Context Detector

**Responsibility**: Provide context about current system state.

**Capabilities**:
- Get active window handle and process ID
- Detect fullscreen applications
- Track mouse position relative to screen bounds

**Fullscreen Detection Logic**:
```rust
fn is_fullscreen() -> bool {
    let foreground = GetForegroundWindow();
    let rect = GetWindowRect(foreground);
    let desktop = GetDesktopWindow();
    let desktop_rect = GetWindowRect(desktop);

    // Must cover entire desktop
    rect == desktop_rect &&
    // Exclude special windows
    !is_special_window(foreground)
}
```

### 6. Configuration Manager

**Responsibility**: Load, save, and validate gesture configuration.

**Configuration Schema** (JSON):
```json
{
  "version": 1,
  "global_gestures": {
    "Right": { "type": "keyboard", "keys": ["VK_BACK"] },
    "Down": { "type": "window", "action": "minimize" }
  },
  "app_gestures": {
    "chrome.exe": {
      "Up": { "type": "keyboard", "keys": ["VK_CONTROL", "T"] }
    }
  },
  "settings": {
    "trigger_button": "Right",
    "min_distance": 20,
    "enable_8_direction": false,
    "disable_in_fullscreen": true
  }
}
```

## Data Flow

### Gesture Recognition Flow
```
1. Mouse Button Down (Hook Thread)
   ↓
2. ShouldPathStart? (Check disabled apps, fullscreen)
   ↓ Yes
3. Start Tracking (Wait for initial_valid_move)
   ↓
4. Mouse Move (Hook Thread)
   ↓
5. Is Effective Move? (Distance > threshold)
   ↓ Yes
6. Parse Gesture Direction (Processing Thread)
   ↓
7. Find Intent (Match gesture → action)
   ↓
8. Intent Recognized Event (Notify UI)
   ↓
9. Modifier Detected (Scroll/Click during gesture)
   ↓
10. Execute Action on Modifier? (If supported)
    ↓
11. Mouse Button Up
    ↓
12. Execute Final Action (If not already executed)
```

### Error Handling Strategy
- **Hook Installation Failure**: Retry with delay, show error dialog, log event
- **Configuration Parse Error**: Use default configuration, show warning in UI
- **Input Simulation Failure**: Log error, continue (don't crash)
- **Gesture Recognition Timeout**: Cancel gesture, simulate original mouse event

## Performance Considerations

### Critical Path Optimization
1. **Hook Callback**: Must return < 1ms to avoid system lag
   - Only capture event, don't process
   - Send to channel and return immediately
2. **Gesture Parsing**: < 5ms per movement event
   - Pre-calculate trigonometric tables for direction calculation
   - Use integer arithmetic where possible
3. **Configuration Lookup**: < 1ms using HashMap

### Memory Management
- Use object pooling for frequently allocated structs (Point, Gesture)
- Limit gesture history to last 1000 points
- Stream configuration loading (don't load entire file into memory)

## Testing Strategy

### Unit Tests
- Direction calculation algorithms
- Gesture matching logic
- Configuration serialization/deserialization

### Integration Tests
- Hook installation/removal
- Input simulation (verify events are generated)
- Full gesture flow (capture → parse → execute)

### Manual Testing
- Test with real applications (browser, file explorer, etc.)
- Verify no interference with normal mouse usage
- Test edge cases (rapid gestures, very slow gestures)

## Security Considerations

1. **Privilege Level**: Run as normal user (no admin required)
2. **Input Simulation**: Always tag simulated events to prevent feedback loops
3. **Configuration**: Validate all loaded config (path traversal, code injection)
4. **Hook Stability**: Ensure hooks are removed on crash/panic (use RAII)

## Future Extensibility

### Planned Enhancements (Out of Scope for v1)
- Gesture recording UI
- Gesture visualization (trace path on screen)
- Plugin system for custom actions
- Cloud sync for configuration
- Macro recording
- Touch/pen gesture support

### Extension Points
- Custom action types (add new Action variants)
- Alternative input sources (touch, pen)
- Advanced gesture algorithms (machine learning)

## References
- WGestures implementation: [GestureParser.cs](https://github.com/yingDev/WGestures/blob/master/WGestures.Core/GestureParser.cs)
- WGestures implementation: [Win32MousePathTracker2.cs](https://github.com/yingDev/WGestures/blob/master/WGestures.Core/Impl/Windows/Win32MousePathTracker2.cs)
- Windows API: [SetWindowsHookEx](https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexa)
- Windows API: [SendInput](https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
