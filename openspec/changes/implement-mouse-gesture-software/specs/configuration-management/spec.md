# Capability: Configuration Management

## ADDED Requirements

### Requirement: Configuration File Structure
The system must load and store gesture configuration in a structured file format.

#### Scenario: Load configuration from JSON file
**Given** a configuration file exists at `config.json`
**When** the application starts
**Then** the configuration should be loaded from the file
**And** the file should contain valid JSON structure
**And** the structure should include: version, global_gestures, app_gestures, settings

#### Scenario: Validate configuration version
**Given** a configuration file is loaded
**When** the version field is missing or incompatible
**Then** the system should log a warning
**And** use default configuration
**And** notify the user of version mismatch

#### Scenario: Handle missing configuration file
**Given** no configuration file exists on startup
**When** the application starts
**Then** a default configuration should be created
**And** the default configuration should be saved to disk
**And** common default gestures should be included (Right→Back, Down→Minimize, etc.)

### Requirement: Gesture Definition Format
The configuration must define gestures and their corresponding actions.

#### Scenario: Define simple gesture
**Given** the configuration file
**When** defining a "Right" gesture
**Then** the gesture should be specified as a sequence: `["Right"]`
**And** the action should be specified with type and parameters
**Example**:
```json
{
  "Right": {
    "type": "keyboard",
    "keys": ["VK_BACK"]
  }
}
```

#### Scenario: Define multi-direction gesture
**Given** the configuration file
**When** defining a "Right → Down" gesture
**Then** the gesture should be specified as: `["Right", "Down"]`
**And** the action should map to this complete sequence

#### Scenario: Define gesture with modifier
**Given** the configuration file
**When** defining a gesture that triggers on scroll wheel
**Then** the modifier should be specified in the gesture definition
**Example**:
```json
{
  "gesture": ["Up"],
  "modifier": "WheelForward",
  "action": {
    "type": "window",
    "command": "maximize"
  }
}
```

### Requirement: Action Definition Format
The configuration must support different action types.

#### Scenario: Define keyboard shortcut action
**Given** a gesture action requires a keyboard shortcut
**When** defining the action in configuration
**Then** the action type should be "keyboard"
**And** the keys should be specified as an array of virtual key codes
**Example**:
```json
{
  "type": "keyboard",
  "keys": ["VK_CONTROL", "VK_C"]
}
```

#### Scenario: Define window command action
**Given** a gesture action requires a window command
**When** defining the action in configuration
**Then** the action type should be "window"
**And** the command should be one of: minimize, maximize, close, restore
**Example**:
```json
{
  "type": "window",
  "command": "minimize"
}
```

#### Scenario: Define mouse action
**Given** a gesture action requires simulating a mouse action
**When** defining the action in configuration
**Then** the action type should be "mouse"
**And** the action should specify button and action type
**Example**:
```json
{
  "type": "mouse",
  "button": "Left",
  "action": "click"
}
```

#### Scenario: Define run command action
**Given** a gesture action should execute a program or command
**When** defining the action in configuration
**Then** the action type should be "run"
**And** the command or executable path should be specified
**Example**:
```json
{
  "type": "run",
  "command": "notepad.exe",
  "args": ""
}
```

### Requirement: Application-Specific Gestures
The configuration must support defining gestures that only apply to specific applications.

#### Scenario: Define app-specific gestures
**Given** the configuration file
**When** defining gestures for a specific application
**Then** the application should be identified by executable name
**And** app-specific gestures should override global gestures
**Example**:
```json
{
  "app_gestures": {
    "chrome.exe": {
      "Up": {
        "type": "keyboard",
        "keys": ["VK_CONTROL", "T"]
      }
    }
  }
}
```

#### Scenario: Multiple applications with different gestures
**Given** multiple applications have app-specific gestures
**When** loading the configuration
**Then** each application should have its own gesture mapping
**And** gestures should be isolated per application

#### Scenario: Global gesture fallback
**Given** an application-specific gesture is not defined
**When** the user performs a gesture in that application
**Then** the global gesture mapping should be used
**And** if no global gesture exists, no action should be executed

### Requirement: Application Exclusion List
The configuration must support disabling gestures for specific applications.

#### Scenario: Disable gestures for application
**Given** the configuration file
**When** adding an application to the disabled list
**Then** the application should be listed in `disabled_apps` array
**And** no gestures should be recognized when this application is active
**Example**:
```json
{
  "disabled_apps": ["notepad.exe", "mspaint.exe"]
}
```

#### Scenario: Validate disabled app format
**Given** the disabled_apps list in configuration
**When** loading the configuration
**Then** each entry should be validated as a valid executable name
**And** invalid entries should be logged and ignored

### Requirement: Settings Configuration
The configuration must support application-wide settings.

#### Scenario: Configure trigger button
**Given** the settings section in configuration
**When** setting the trigger button
**Then** the button should be one of: Right, Middle, X1, X2
**And** the trigger button should be used to start gesture tracking
**Example**:
```json
{
  "settings": {
    "trigger_button": "Right"
  }
}
```

#### Scenario: Configure gesture sensitivity
**Given** the settings section in configuration
**When** setting gesture sensitivity parameters
**Then** the following should be configurable:
  - `min_distance`: pixels before gesture starts (default: 5)
  - `effective_move`: pixels for gesture direction (default: 20)
  - `stay_timeout`: milliseconds before gesture timeout (default: 500)
**Example**:
```json
{
  "settings": {
    "min_distance": 10,
    "effective_move": 30,
    "stay_timeout": 300
  }
}
```

#### Scenario: Configure advanced options
**Given** the settings section in configuration
**When** setting advanced options
**Then** the following should be configurable:
  - `enable_8_direction`: boolean for 8-direction gestures
  - `disable_in_fullscreen`: boolean to disable in fullscreen apps
**Example**:
```json
{
  "settings": {
    "enable_8_direction": false,
    "disable_in_fullscreen": true
  }
}
```

### Requirement: Configuration Persistence
The system must save configuration changes to disk.

#### Scenario: Save configuration on modification
**Given** the configuration is modified (e.g., via UI)
**When** the modification is complete
**Then** the configuration should be saved to the config file
**And** the file should be atomically written (write to temp, then rename)
**And** a backup of the old configuration should be created

#### Scenario: Create configuration backup
**Given** a configuration save is about to happen
**When** an existing configuration file exists
**Then** the old file should be copied to `config.json.backup`
**And** only one backup should be maintained (overwrite previous backup)

#### Scenario: Handle save failure gracefully
**Given** the configuration cannot be saved (disk full, permissions)
**When** the save attempt fails
**Then** the error should be logged
**And** the user should be notified via UI if available
**And** the in-memory configuration should remain valid

### Requirement: Configuration Hot Reload
The system should detect and reload configuration changes.

#### Scenario: Detect external configuration changes
**Given** the application is running
**When** the configuration file is modified externally
**Then** the application should detect the file change
**And** reload the configuration within 1 second
**And** apply the new configuration to subsequent gestures

#### Scenario: Validate configuration on reload
**Given** a configuration file is reloaded
**When** the file contains invalid JSON or schema
**Then** the reload should fail gracefully
**And** the old configuration should remain in use
**And** an error should be logged

### Requirement: Configuration Import/Export
The system should support importing and exporting configurations.

#### Scenario: Export configuration to file
**Given** the user wants to export their configuration
**When** the export action is triggered
**Then** a file save dialog should be shown (if UI available)
**And** the current configuration should be written to the chosen location
**And** the exported file should be in JSON format

#### Scenario: Import configuration from file
**Given** the user wants to import a configuration
**When** an import file is selected
**Then** the file should be validated for correct format
**And** the configuration should be loaded into memory
**And** the user should be prompted to merge or replace existing config
**And** the imported configuration should be saved

#### Scenario: Validate imported configuration
**Given** a configuration file is being imported
**When** the file is loaded
**Then** all gestures should be validated
**And** all actions should be validated
**And** a summary of validation errors should be shown
**And** the import should be aborted if critical errors exist

### Requirement: Configuration Migration
The system should handle configuration schema changes between versions.

#### Scenario: Migrate from older version
**Given** a configuration file from version 1 is loaded
**When** the current version is 2
**Then** the system should recognize the version mismatch
**And** apply migration rules to convert v1 → v2 format
**And** save the migrated configuration
**And** notify the user of the migration

#### Scenario: Handle unknown future version
**Given** a configuration file from version 3 is loaded
**When** the current version is 2
**Then** the system should detect the higher version
**And** log a warning about future version
**And** attempt to load with backwards compatibility
**And** fall back to default config if load fails
