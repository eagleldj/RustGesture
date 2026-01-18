# Proposal: Implement Mouse Gesture Software

## Change ID
`implement-mouse-gesture-software`

## Status
**Proposed** | 2026-01-18

## Overview
Develop a modern mouse gesture software for Windows in Rust, inspired by WGestures. The application will recognize mouse gestures and execute corresponding actions, providing an intuitive way to control Windows through mouse movements.

## Motivation
WGestures is a popular mouse gesture software written in C#/.NET. This project aims to recreate similar functionality using Rust to achieve:
- **Better performance** - Rust's zero-cost abstractions and memory safety
- **Smaller binary size** - No runtime dependency unlike .NET
- **Modern architecture** - Learn from WGestures' design while improving upon it
- **Cross-platform potential** - Rust enables future macOS/Linux support

## Scope

### In Scope
1. **Core Gesture Recognition System**
   - Low-level mouse hook implementation using Windows API
   - Real-time mouse path tracking and gesture parsing
   - 4-direction and 8-direction gesture recognition
   - Gesture modifiers (scroll wheel, additional mouse buttons)

2. **Gesture Configuration**
   - Gesture-to-action mapping system
   - Application-specific gesture rules
   - Configuration persistence

3. **Action Execution**
   - Keyboard input simulation
   - Mouse input simulation
   - Windows command execution (e.g., minimize, close, etc.)

4. **User Interface**
   - Global hotkey support
   - System tray icon and context menu
   - Basic settings UI for gesture configuration

### Out of Scope
- Gesture recording/training UI (deferred to future proposal)
- Cloud sync/configuration backup
- Advanced gesture visualization
- Plugin system
- Screen corner detection (hot corners)
- Screen edge rubbing detection

## Affected Capabilities
This proposal introduces the following new capabilities:
- `gesture-recognition` - Core gesture recognition and path tracking
- `input-simulation` - Mouse and keyboard input simulation
- `configuration-management` - Gesture configuration and persistence
- `ui-management` - System tray and basic settings UI

## Success Criteria
1. User can perform mouse gestures (e.g., Right → Down) and they are correctly recognized
2. Gestures trigger configured actions (keyboard shortcuts, window management, etc.)
3. Application runs as background service with tray icon
4. Configuration can be edited and persisted across restarts
5. Performance: < 10ms latency from gesture completion to action execution
6. Memory usage: < 50MB when idle

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Windows API complexity in Rust | High | Use proven crates (windows-rs, winapi) and reference WGestures implementation |
| Input simulation conflicts | Medium | Implement event tagging to distinguish simulated from real input |
| Gesture recognition accuracy | Medium | Implement configurable sensitivity thresholds |
| Accessibility issues | Low | Ensure gestures don't interfere with screen readers/assistive tech |

## Dependencies
- Windows OS (Windows 10+)
- Rust 2024 edition
- External crates:
  - `windows` crate for Win32 API bindings
  - `serde` for configuration serialization
  - `tokio` for async runtime
  - `tray-icon` or similar for system tray

## Open Questions
1. Should we support configuration via JSON, TOML, or a custom format?
   - **Recommendation**: Start with JSON for simplicity, consider TOML later
2. What's the minimum viable UI for v1.0?
   - **Recommendation**: System tray with menu (Enable/Disable/Settings/Exit)
3. Should gestures be globally disabled in fullscreen applications?
   - **Recommendation**: Yes, add option to disable in fullscreen mode

## Related Proposals
None - this is the foundational proposal for the project.

## References
- [WGestures Source Code](https://github.com/yingDev/WGestures)
- [WGestures Documentation](http://www.yingdev.com/projects/wgestures)
- Windows API documentation for mouse hooks and input simulation
