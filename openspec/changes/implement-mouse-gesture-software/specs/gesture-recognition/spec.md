# Capability: Gesture Recognition

## ADDED Requirements

### Requirement: Low-Level Mouse Input Capture
The system must capture low-level mouse input events globally across all applications using Windows hooks.

#### Scenario: Install global mouse hook
**Given** the application is running
**When** the gesture recognition system initializes
**Then** a low-level mouse hook should be installed using `SetWindowsHookEx` with `WH_MOUSE_LL`
**And** the hook should receive all mouse events system-wide
**And** the hook callback should execute in a dedicated thread

#### Scenario: Filter non-gesture mouse events
**Given** the mouse hook is installed
**When** a mouse button (Right/Middle/X1/X2) is pressed
**And** the application is in the disabled list
**Then** the mouse event should pass through to the target application unchanged
**And** no gesture tracking should start

#### Scenario: Identify gesture trigger button
**Given** the mouse hook is installed
**When** the Right mouse button is pressed
**And** Right button is configured as trigger button
**Then** the system should evaluate whether to start gesture tracking
**When** the Middle mouse button is pressed
**And** Middle button is configured as trigger button
**Then** the system should evaluate whether to start gesture tracking

### Requirement: Gesture Path Tracking
The system must track mouse movement paths after trigger button press and convert them into gesture directions.

#### Scenario: Start tracking after initial movement threshold
**Given** a trigger mouse button is pressed
**And** the system has decided to capture the gesture
**When** the mouse moves less than 5 pixels from the start point
**Then** gesture tracking should NOT start
**When** the mouse moves more than 5 pixels from the start point
**Then** gesture tracking should start
**And** the initial position should be recorded as the gesture start point

#### Scenario: Detect effective movement
**Given** gesture tracking has started
**When** the mouse moves less than the effective move threshold (20 pixels)
**Then** no new gesture direction should be recorded
**When** the mouse moves more than the effective move threshold
**Then** a new gesture direction should be calculated and recorded

#### Scenario: Track 4-direction gestures
**Given** gesture tracking is active
**When** the mouse moves primarily horizontally (|dx| > |dy|)
**And** dx > 0 (moving right)
**Then** the gesture direction should be "Right"
**When** dx < 0 (moving left)
**Then** the gesture direction should be "Left"
**When** the mouse moves primarily vertically (|dy| > |dx|)
**And** dy < 0 (moving up, due to screen coordinates)
**Then** the gesture direction should be "Up"
**When** dy > 0 (moving down)
**Then** the gesture direction should be "Down"

#### Scenario: Track 8-direction gestures (first stroke only)
**Given** 8-direction gesture mode is enabled
**And** this is the first stroke of the gesture
**When** the mouse moves at a 45-degree angle (up-right)
**Then** the gesture direction should be diagonal "Up-Right"
**When** the mouse moves at a 135-degree angle (up-left)
**Then** the gesture direction should be diagonal "Up-Left"
**When** the mouse moves at a 225-degree angle (down-left)
**Then** the gesture direction should be diagonal "Down-Left"
**When** the mouse moves at a 315-degree angle (down-right)
**Then** the gesture direction should be diagonal "Down-Right"

#### Scenario: Subsequent strokes use 4-direction
**Given** 8-direction gesture mode is enabled
**And** the gesture already has at least one direction
**When** the mouse changes direction
**Then** only 4-direction recognition should be used (no diagonals)
**And** any diagonal from the first stroke should be converted to its dominant axis

### Requirement: Gesture Modifiers
The system must recognize gesture modifiers (scroll wheel, additional mouse buttons) during gesture tracking.

#### Scenario: Recognize scroll wheel modifier
**Given** a gesture is being tracked
**When** the mouse wheel scrolls forward
**Then** a "Wheel Forward" modifier should be detected
**When** the mouse wheel scrolls backward
**Then** a "Wheel Backward" modifier should be detected
**And** modifier events should be throttled to at most 1 event per 100ms

#### Scenario: Recognize additional button modifiers
**Given** a gesture is being tracked with Right mouse button
**When** the Left mouse button is pressed
**Then** a "Left Button Down" modifier should be detected
**When** Middle mouse button is pressed (if not trigger button)
**Then** a "Middle Button Down" modifier should be detected

### Requirement: Gesture Completion
The system must detect when a gesture is complete and trigger action execution.

#### Scenario: Complete gesture on trigger button release
**Given** a gesture is being tracked
**When** the trigger mouse button is released
**Then** the gesture should be marked as complete
**And** the gesture should be matched against configured actions
**And** the matched action should be executed

#### Scenario: Cancel gesture without sufficient movement
**Given** a trigger mouse button is pressed
**When** the mouse is released before moving 5 pixels
**Then** no gesture should be recorded
**And** the original mouse click should be simulated to the target application

#### Scenario: Cancel gesture on timeout
**Given** a gesture is being tracked
**When** no effective movement occurs for 500ms (stay timeout enabled)
**Then** the gesture should be cancelled
**And** a timeout event should be raised

### Requirement: Context-Aware Gesture Recognition
The system must detect application context and apply app-specific gesture rules.

#### Scenario: Detect active application
**Given** a gesture is about to start
**When** checking the current context
**Then** the active window's process ID should be retrieved
**And** the process executable name should be identified

#### Scenario: Apply application-specific gestures
**Given** the active application is "chrome.exe"
**And** a global gesture "Right" maps to "Back" action
**And** a chrome-specific gesture "Right" maps to "Next Tab" action
**When** the user performs a "Right" gesture
**Then** the chrome-specific action ("Next Tab") should take precedence
**And** the global action should NOT be executed

#### Scenario: Disable gestures in fullscreen applications
**Given** fullscreen detection is enabled
**When** the active application is in fullscreen mode
**Then** no gesture tracking should start
**And** all mouse events should pass through normally

#### Scenario: Disable gestures for specific applications
**Given** "notepad.exe" is in the disabled applications list
**When** the user tries to start a gesture in Notepad
**Then** gesture tracking should NOT start
**And** the mouse events should pass through normally

### Requirement: Performance and Reliability
The gesture recognition system must meet performance requirements and handle edge cases gracefully.

#### Scenario: Process hook callback with minimal latency
**Given** a low-level mouse hook callback is executing
**When** processing the mouse event
**Then** the callback should return within 1 millisecond
**And** the event should be queued for processing rather than processed synchronously

#### Scenario: Handle simulated input events
**Given** the system has simulated a mouse/keyboard event
**When** the simulated event is received by the hook callback
**Then** the event should be ignored (not processed as a gesture)
**And** the `dwExtraInfo` field should be checked for the simulated event tag

#### Scenario: Gracefully handle hook installation failure
**Given** the application is starting
**When** hook installation fails (e.g., another hook is blocking)
**Then** the application should log the error
**And** display a user-friendly error message
**And** continue running with gesture recognition disabled

#### Scenario: Limit gesture complexity
**Given** a gesture is being tracked
**When** the gesture reaches 12 direction changes (max steps)
**Then** no additional directions should be recorded
**And** the gesture should be limited to the first 12 directions
