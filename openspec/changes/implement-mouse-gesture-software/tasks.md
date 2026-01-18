# Tasks: Implement Mouse Gesture Software

## Overview
This document breaks down the implementation of the mouse gesture software into ordered, verifiable work items.

## Phase 1: Foundation (Week 1-2)

### 1.1 Project Setup and Build System
- [ ] Create Cargo.toml with required dependencies (windows, serde, tokio, etc.)
- [ ] Set up workspace structure (src/bin, src/lib, crates/)
- [ ] Configure build script for Windows resource embedding (icon, manifest)
- [ ] Create basic `main.rs` with application entry point
- [ ] Set up logging infrastructure (tracing crate)
- [ ] **Validation**: `cargo build` succeeds, binary runs and exits cleanly

### 1.2 Configuration Data Structures
- [ ] Define `Gesture` struct (sequence of directions, optional modifiers)
- [ ] Define `GestureDir` enum (Up, Down, Left, Right, diagonals)
- [ ] Define `GestureModifier` enum (LeftButtonDown, RightButtonDown, WheelForward, etc.)
- [ ] Define `Action` enum (Keyboard, Mouse, WindowCommand, RunCommand)
- [ ] Define `GestureConfig` struct (global gestures, app-specific, settings)
- [ ] Implement Serde serialization/deserialization for all config types
- [ ] **Validation**: Unit tests serialize and deserialize sample config JSON

### 1.3 Configuration File Management
- [ ] Implement `ConfigManager` to load config from default path
- [ ] Create default configuration with sample gestures
- [ ] Implement config validation (version check, required fields)
- [ ] Implement config saving with atomic write (temp file + rename)
- [ ] Implement backup creation before save
- [ ] Add error handling for invalid JSON and missing files
- [ ] **Validation**: Manual test - create/edit config file, verify it loads

### 1.4 Windows API Bindings
- [ ] Create `winapi` module with required Windows API imports
- [ ] Bind `SetWindowsHookExW` for low-level mouse/keyboard hooks
- [ ] Bind `UnhookWindowsHookEx` for cleanup
- [ ] Bind `SendInput` for input simulation
- [ ] Bind `GetAsyncKeyState` for key state checking
- [ ] Bind `GetForegroundWindow`, `GetWindowThreadProcessId` for context
- [ ] Bind `SetCursorPos`, `GetCursorPos` for mouse positioning
- [ ] **Validation**: Test calls compile and don't panic at runtime

## Phase 2: Input Capture (Week 2-3)

### 2.1 Low-Level Mouse Hook
- [ ] Implement `MouseKeyboardHook` struct to manage Windows hooks
- [ ] Create hook callback procedure (mouse events)
- [ ] Implement hook installation in dedicated thread
- [ ] Create message queue to serialize hook events
- [ ] Implement safe event passing to main thread (channels)
- [ ] Add error handling for hook installation failures
- [ ] **Validation**: Hook installs successfully, events are received

### 2.2 Mouse Event Processing
- [ ] Define `MouseHookEventArgs` struct (message, x, y, extra_info)
- [ ] Implement mouse button down detection (Right, Middle, X1, X2)
- [ ] Implement mouse button up detection
- [ ] Implement mouse move tracking
- [ ] Implement mouse wheel detection (scroll delta)
- [ ] Filter out simulated events using dwExtraInfo tag
- [ ] **Validation**: Log mouse events, verify simulated events are filtered

### 2.3 Capture Decision Logic
- [ ] Implement `should_start_gesture()` function
- [ ] Check if trigger button is pressed
- [ ] Check if current app is in disabled list
- [ ] Implement fullscreen detection (window rect == desktop rect)
- [ ] Exclude special windows (desktop, shell, immersive launcher)
- [ ] Add context detection (active window, process ID)
- [ ] **Validation**: Unit tests for capture decision with mock contexts

### 2.4 Initial Movement Detection
- [ ] Record start point on mouse button down
- [ ] Calculate distance from start point on each move
- [ ] Implement `initial_valid_move` threshold (5 pixels)
- [ ] Handle initial stay timeout (150ms no movement)
- [ ] Simulate original click if gesture doesn't start
- [ ] **Validation**: Manual test - press trigger, don't move, verify click passes through

## Phase 3: Gesture Recognition (Week 3-4)

### 3.1 Path Tracking
- [ ] Implement `PathTracker` trait and `Win32MousePathTracker` struct
- [ ] Start tracking after initial valid movement
- [ ] Record mouse movement points
- [ ] Implement `effective_move` threshold (20 pixels)
- [ ] Emit `PathStart` event when tracking starts
- [ ] Emit `PathGrow` event on every move (for UI feedback)
- [ ] Emit `EffectivePathGrow` event when threshold is crossed
- [ ] **Validation**: Log path tracking events, verify thresholds

### 3.2 Direction Calculation (4-Direction)
- [ ] Implement `calculate_4direction()` function
- [ ] Calculate vector from last point to current point
- [ ] Determine quadrant based on (dx, dy)
- [ ] Compare |dx| vs |dy| to pick primary axis
- [ ] Return Up, Down, Left, or Right
- [ ] Add unit tests for all quadrants and edge cases
- [ ] **Validation**: Unit tests with known vectors

### 3.3 Direction Calculation (8-Direction)
- [ ] Implement `calculate_8direction()` function
- [ ] Calculate angle of movement vector using atan2
- [ ] Divide 360° into 8 sectors (45° each)
- [ ] Implement fuzzy matching for diagonal detection (50% range)
- [ ] Handle first-stroke diagonal to 4-direction conversion
- [ ] Add unit tests for all 8 directions
- [ ] **Validation**: Unit tests with known angles

### 3.4 Gesture State Machine
- [ ] Implement `GestureParser` struct
- [ ] Maintain gesture state (list of directions, modifiers)
- [ ] Add new direction only when it differs from last direction
- [ ] Limit gesture to max steps (12 directions)
- [ ] Emit `GestureChanged` event when direction is added
- [ ] Reset state on gesture completion or cancellation
- [ ] **Validation**: Log state transitions, verify gesture sequences

### 3.5 Gesture Modifiers
- [ ] Detect scroll wheel events during gesture
- [ ] Detect additional mouse button presses during gesture
- [ ] Throttle modifier events (max 1 per 100ms)
- [ ] Emit `GestureModifier` event with modifier type
- [ ] Implement modifier filtering for actions that need real-time feedback
- [ ] **Validation**: Manual test - perform gesture while scrolling

### 3.6 Gesture Completion
- [ ] Detect trigger button release
- [ ] Emit `GestureEnd` event with final gesture data
- [ ] Handle incomplete gestures (no initial movement)
- [ ] Implement stay timeout (500ms no movement)
- [ ] Cancel gesture on timeout and emit `GestureTimeout` event
- [ ] **Validation**: Manual test - complete gesture, verify end event

## Phase 4: Gesture Matching (Week 4)

### 4.1 Gesture Intent Finder
- [ ] Implement `GestureIntentFinder` struct
- [ ] Create `GestureIntent` struct (gesture, action, context)
- [ ] Implement gesture lookup in global config
- [ ] Implement gesture lookup in app-specific config
- [ ] Priority: app-specific → global
- [ ] Handle exact matches only (no partial matches in v1)
- [ ] **Validation**: Unit tests with mock configs

### 4.2 Context-Aware Matching
- [ ] Get active window handle and process ID
- [ ] Resolve process ID to executable name
- [ ] Check if app has custom gesture rules
- [ ] Check if app is in disabled list
- [ ] Cache process name to avoid repeated lookups
- [ ] **Validation**: Manual test - different gestures in different apps

### 4.3 Gesture Recognition Events
- [ ] Emit `IntentRecognized` event when gesture is matched
- [ ] Emit `IntentInvalid` event when gesture doesn't match
- [ ] Support event subscriptions (UI for visual feedback)
- [ ] Thread-safe event dispatch
- [ ] **Validation**: UI shows recognized gesture in real-time

## Phase 5: Input Simulation (Week 5)

### 5.1 Keyboard Input Simulation
- [ ] Implement `KeyboardSimulator` struct
- [ ] Implement `send_key_press()` (keydown + keyup)
- [ ] Implement `send_key_combination()` (modifiers + target)
- [ ] Map virtual key codes (VK_*) to Windows scan codes
- [ ] Support extended keys flag
- [ ] Tag all simulated events with dwExtraInfo
- [ ] **Validation**: Manual test - simulate Alt+Tab, verify it works

### 5.2 Mouse Input Simulation
- [ ] Implement `MouseSimulator` struct
- [ ] Implement `send_click()` (mousedown + mouseup with delay)
- [ ] Implement `send_double_click()` (two clicks with system delay)
- [ ] Implement `set_cursor_position()` using SetCursorPos
- [ ] Implement `send_mouse_wheel()` with delta
- [ ] Tag all simulated events with dwExtraInfo
- [ ] **Validation**: Manual test - simulate clicks, verify they work

### 5.3 Window Commands
- [ ] Implement `WindowCommand` executor
- [ ] Minimize: Send Win+Down or ShowWindow(SW_MINIMIZE)
- [ ] Maximize: Send Win+Up or ShowWindow(SW_MAXIMIZE)
- [ ] Close: Send Alt+F4 or WM_CLOSE message
- [ ] Restore: Send Win+Shift+Up or ShowWindow(SW_RESTORE)
- [ ] Show Desktop: Send Win+D or toggle desktop
- [ ] **Validation**: Manual test - execute window commands

### 5.4 Run Command Action
- [ ] Implement `CommandExecutor` struct
- [ ] Use `CreateProcess` or `std::process::Command`
- [ ] Support executable path and arguments
- [ ] Handle command failures (log error, don't crash)
- [ ] **Validation**: Manual test - run notepad.exe

### 5.5 Device State Checking
- [ ] Implement `InputDeviceState` checker
- [ ] Check async key state before simulating
- [ ] Avoid releasing already-pressed modifier keys
- [ ] Handle partial modifier state (e.g., Ctrl already held)
- [ ] **Validation**: Manual test - gesture while holding Ctrl

## Phase 6: System Tray and UI (Week 5-6)

### 6.1 System Tray Icon
- [ ] Add `tray-icon` crate dependency
- [ ] Create tray icon from embedded resource
- [ ] Display tooltip "RustGesture"
- [ ] Implement context menu (Enable/Disable, Settings, About, Exit)
- [ ] Handle menu item clicks
- [ ] **Validation**: Tray icon appears, menu works

### 6.2 Toggle Enable/Disable
- [ ] Implement enabled/disabled state
- [ ] When disabled, don't install hooks or process gestures
- [ ] Update tray menu to show current state
- [ ] Persist enabled state to config
- [ ] **Validation**: Toggle from tray, verify gestures stop/start

### 6.3 Settings Window (Basic)
- [ ] Choose UI framework (egui/tauri/Slint - recommend tauri for v1)
- [ ] Create settings window layout
- [ ] Implement gesture list view
- [ ] Implement Add/Edit/Delete buttons
- [ ] Implement tab organization (Gestures, General, Advanced)
- [ ] **Validation**: Window opens, shows configured gestures

### 6.4 Gesture Editor
- [ ] Create gesture editor dialog
- [ ] Implement gesture recording mode (capture from PathTracker)
- [ ] Implement manual direction selector
- [ ] Implement action selector (type + parameters)
- [ ] Implement scope selector (global/app-specific)
- [ ] **Validation**: Record gesture, save it, verify it works

### 6.5 Settings Persistence
- [ ] Load settings on window open
- [ ] Apply settings on "OK" or "Apply"
- [ ] Save window position and size
- [ ] Save last selected tab
- [ ] **Validation**: Change settings, close/reopen, verify persisted

### 6.6 About Dialog
- [ ] Create about dialog
- [ ] Show version, author, license
- [ ] Add link to repository
- [ ] **Validation**: Dialog opens and displays info

### 6.7 Notifications
- [ ] Add notification system (Windows toast or custom)
- [ ] Show notification on gesture execution (optional)
- [ ] Show notification on errors
- [ ] Show notification on config reload
- [ ] Add settings to disable notifications
- [ ] **Validation**: Perform gesture, see notification

## Phase 7: Polish and Testing (Week 6-7)

### 7.1 Error Handling
- [ ] Add error logging for all failure modes
- [ ] Implement graceful degradation on hook failure
- [ ] Show user-friendly error messages
- [ ] Add crash handler (minidump generation)
- [ ] **Validation**: Test various error scenarios

### 7.2 Performance Optimization
- [ ] Profile hook callback (target < 1ms)
- [ ] Profile gesture parsing (target < 5ms)
- [ ] Profile config lookup (target < 1ms)
- [ ] Optimize hot paths if needed
- [ ] **Validation**: Measure latencies, verify targets

### 7.3 Comprehensive Testing
- [ ] Unit tests for direction calculation (4 and 8 direction)
- [ ] Unit tests for gesture matching logic
- [ ] Unit tests for config serialization
- [ ] Integration tests for full gesture flow
- [ ] Manual testing with real applications
- [ ] **Validation**: All tests pass, manual testing checklist complete

### 7.4 Documentation
- [ ] Write README with features and usage
- [ ] Create user guide (how to perform gestures)
- [ ] Document configuration format
- [ ] Add inline code documentation
- [ ] **Validation**: Docs are clear and complete

### 7.5 Packaging and Distribution
- [ ] Create installer (NSIS or WiX)
- [ ] Configure digital signing (optional)
- [ ] Test clean install on fresh Windows
- [ ] Test upgrade from previous version
- [ ] Create release notes
- [ ] **Validation**: Install on clean machine, verify it works

### 7.6 Release Preparation
- [ ] Version tagging (git tag)
- [ ] Build release binary (optimized, stripped)
- [ ] Verify all features work
- [ ] Final testing pass
- [ ] Deploy to GitHub Releases or website
- [ ] **Validation**: Binary runs, no obvious bugs

## Dependencies and Parallelization

### Can be done in parallel:
- Tasks 1.2 and 1.4 (data structures and API bindings)
- Tasks 2.1-2.4 (all input capture tasks can iterate)
- Tasks 3.2 and 3.3 (4-direction and 8-direction algorithms)
- Tasks 5.1-5.4 (all input simulation types)
- Tasks 6.3-6.6 (all UI components)

### Must be sequential:
- Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 (core pipeline)
- Phase 3.1 must complete before 3.2-3.6
- Task 4.1 must complete before 4.2
- Phase 6 must wait for Phase 5 (UI needs core features)

## Definition of Done
Each task is complete when:
- Code is written and compiles without warnings
- Tests pass (unit tests for logic, manual for UI)
- Code is reviewed (self-review or pair review)
- Documentation is updated (if needed)
- Task is marked as done in this checklist
