# Capability: Input Simulation

## ADDED Requirements

### Requirement: Keyboard Input Simulation
The system must simulate keyboard input to execute keyboard shortcut actions.

#### Scenario: Simulate single key press
**Given** a gesture action requires simulating the "Backspace" key
**When** the action is executed
**Then** a `KEYBDINPUT` event should be sent with `VK_BACK` key down
**And** a subsequent `KEYBDINPUT` event should be sent with `VK_BACK` key up
**And** the simulated events should be tagged in `dwExtraInfo`

#### Scenario: Simulate key combination (Ctrl+C)
**Given** a gesture action requires simulating Ctrl+C
**When** the action is executed
**Then** `VK_CONTROL` key down event should be sent
**And** `VK_C` key down event should be sent
**And** `VK_C` key up event should be sent
**And** `VK_CONTROL` key up event should be sent
**And** all events should be sent in a single `SendInput` call

#### Scenario: Simulate key combination with multiple modifiers (Ctrl+Shift+T)
**Given** a gesture action requires simulating Ctrl+Shift+T
**When** the action is executed
**Then** modifier keys should be pressed in order: Control, Shift
**And** the target key should be pressed: T
**And** keys should be released in reverse order: T, Shift, Control

#### Scenario: Simulate extended keys
**Given** a gesture action requires simulating the F1 key
**When** the action is executed
**Then** the keyboard input should include the extended key flag
**And** the correct virtual key code for F1 should be used

### Requirement: Mouse Input Simulation
The system must simulate mouse input to execute mouse-related actions.

#### Scenario: Simulate mouse click
**Given** a gesture action requires simulating a left mouse click
**When** the action is executed
**Then** a `MOUSEEVENTF_LEFTDOWN` event should be sent
**And** a `MOUSEEVENTF_LEFTUP` event should be sent
**And** events should be separated by 10ms (click press-release interval)

#### Scenario: Simulate mouse double-click
**Given** a gesture action requires simulating a double-click
**When** the action is executed
**Then** two complete click sequences should be sent
**And** clicks should be separated by the system double-click time

#### Scenario: Simulate mouse movement
**Given** a gesture action requires moving the mouse to specific coordinates
**When** the action is executed
**Then** `SetCursorPos` should be called with the target x,y coordinates
**And** the mouse should move to the specified screen location

#### Scenario: Simulate mouse wheel scroll
**Given** a gesture action requires scrolling up
**When** the action is executed
**Then** a `MOUSEEVENTF_WHEEL` event should be sent
**And** the wheel delta should be positive (e.g., WHEEL_DELTA = 120)
**When** a gesture action requires scrolling down
**Then** the wheel delta should be negative (e.g., -120)

### Requirement: Window Command Simulation
The system must execute window management commands using simulated input or Windows API calls.

#### Scenario: Minimize active window
**Given** a gesture action requires minimizing the current window
**When** the action is executed
**Then** the system should simulate Win+Down (or Win+Down, Down for Windows 11)
**Or** call `ShowWindow` with `SW_MINIMIZE` on the active window

#### Scenario: Maximize active window
**Given** a gesture action requires maximizing the current window
**When** the action is executed
**Then** the system should simulate Win+Up
**Or** call `ShowWindow` with `SW_MAXIMIZE` on the active window

#### Scenario: Close active window
**Given** a gesture action requires closing the current window
**When** the action is executed
**Then** the system should simulate Alt+F4
**Or** send `WM_CLOSE` message to the active window

#### Scenario: Show desktop
**Given** a gesture action requires showing the desktop
**When** the action is executed
**Then** the system should simulate Win+D
**Or** toggle the desktop with appropriate Windows API

### Requirement: Input Simulation Safety
The system must ensure simulated input doesn't interfere with gesture recognition.

#### Scenario: Tag simulated input events
**Given** the system is simulating a keyboard or mouse event
**When** the `SendInput` API is called
**Then** every simulated event should have a unique tag in `dwExtraInfo`
**And** the tag value should be a constant (e.g., 19900620)
**And** this tag should match the filter in the mouse hook callback

#### Scenario: Prevent feedback loops
**Given** the system has just simulated a mouse click
**When** the hook callback receives the simulated event
**Then** the `dwExtraInfo` field should be checked
**And** if it matches the simulation tag, the event should be ignored
**And** no gesture tracking should start from simulated events

#### Scenario: Delay hook processing after simulation
**Given** the system has simulated a complex key combination
**When** the simulation completes
**Then** gesture recognition should remain suspended for 300ms
**And** this prevents the simulated events from being interpreted as gestures

### Requirement: Device State Awareness
The system must check current input device state before simulating input.

#### Scenario: Check current key states
**Given** a gesture action requires simulating Ctrl+C
**When** the action is about to execute
**Then** the system should check if Ctrl is already pressed
**And** if Ctrl is pressed, only simulate C press/release
**And** avoid releasing the existing Ctrl state

#### Scenario: Check current mouse button states
**Given** a gesture action requires simulating a mouse click
**When** the action is about to execute
**Then** the system should check if the target mouse button is already pressed
**And** avoid button state conflicts

#### Scenario: Query async key state
**Given** the system needs to check if a key is pressed
**When** `GetAsyncKeyState` is called
**Then** the return value should indicate if the key is currently down
**And** the result should be used to avoid conflicting input simulations

### Requirement: Input Simulation Error Handling
The system must handle simulation failures gracefully.

#### Scenario: Handle SendInput failure
**Given** `SendInput` is called to simulate input
**When** the API call fails (returns 0)
**Then** the error should be logged
**And** the gesture action should be marked as failed
**And** the system should continue running (don't crash)

#### Scenario: Handle invalid virtual key codes
**Given** a gesture configuration contains an invalid key code
**When** the action is executed
**Then** the simulation should fail gracefully
**And** an error should be logged with the invalid key code
**And** the user should be notified if UI is available

#### Scenario: Handle SetCursorPos failure
**Given** the system tries to move the cursor
**When** `SetCursorPos` fails (e.g., invalid coordinates)
**Then** the error should be logged
**And** the action should be marked as failed

### Requirement: Input Simulation Performance
The system must execute simulated input efficiently.

#### Scenario: Batch multiple input events
**Given** a gesture action requires sending multiple key events
**When** the action is executed
**Then** all events should be sent in a single `SendInput` call
**And** the array size should not exceed the maximum (typically 64 events)

#### Scenario: Minimize latency between gesture and execution
**Given** a gesture has just been recognized
**When** the action execution starts
**Then** input simulation should begin within 10ms
**And** the complete action should finish within 50ms for simple shortcuts

#### Scenario: Avoid blocking UI thread during simulation
**Given** an input simulation is in progress
**When** the simulation executes
**Then** it should run on a background thread if possible
**Or** complete quickly enough to not freeze the UI (< 100ms)
