# bevy_basic_prefs

This crate provides basic preferences support for Bevy applications. The word "preferences"
in this context is used to mean user settings that are (1) set while running the app, (2) persistent
across restarts, and (3) implicitly saved. It is not meant to be a general config file
serialization mechanism.

Preferences typically include things like:

- Current editing "mode" or tool.
- Keyboard or game controller bindings.
- Music and sound effects volume settings.
- The location of the last saved game.
- The user's login name for a network game (but not password!)
- "Do not show this dialog again" checkbox settings.

Preferences are _NOT_:

- **Saved games**. The user can have many saved games, wherease typically there is only one set of
  preferences, which is user global. Also, while many games require the user to explicitly perform
  a "save" action, preferences generally are saved automatically.
- **Assets**. Preferences live in the operating-system-specific folder for user settings,
  whereas assets are something that is shipped with the game.
- **Meant to be human-editable**. While it is possible to edit preference files, these files are
  located in a system folder that is "hidden" from non-technical users such as `~/.config` or
  `$HOME/Library/Preferences/`. That being said, the format of preference files is TOML, which
  can easily be edited in a text editor.
- **Meant to be editable by other applications** - this crate only supports "basic" preferences,
  which means that it intentionally does not support some of the more advanced use cases. This
  includes cases where a third-party tool writes out a config file which is read by the game.

## Supported Features

- Preferences are serialized to TOML format.
- Any reflectable resource can be saved as a preference by annotating it with a special preference
  annotation. (Currently only resources are supported, however this could be extended if there
  is sufficient interest.)
- Supports storing Bevy states as preferences, and automatically restoring those states on startup.
- Flexible "grouping" annotations allows related preferences to be grouped together in the settings
  file, even if they are in different resources.
- Simplified serialization: "wrapper" types, such as newtype structs, are stripped from the output,
  making the resulting settings file more readable.
- Preferences are saved in standard OS locations. Config directories are created if they do
  not already exist. The settings directory name is configurable.
- File-corruption-resistant: the framework will save the settings to a temp file, close the file,
  and then use a filesystem operation to move the temporary file to the settings config. This means
  that if the game crashes while saving, the settings file won't be corrupted.
- Debouncing/throttling - often a user setting, such as an audio volume slider or window
  splitter bar, changes at high frequency when dragged. The library allows you to mark preferences
  as "changed", which will save out preferences after a delay of one second.
- Various configurable options for saving preferences:
  - Fully-automatic: the preferences system will watch for changes to the preference resources,
    and queue a deferred save action.
  - Mark changed: you can explicitly mark the preferences as "changed", which will trigger a
    deferred save.
  - Explicit synchronous flush: you can issue a `Command` which immediately and synchronously
    writes out the settings file.

## Planned features

- Web support. Currently the library uses filesystem operations, but it could be extended to
  support browser local storage (possibly using JSON format instead of TOML since that is
  more web-idiomatic).
- Support for more data types:
  - Option
  - AssetPath / AssetId / Handle
  - Tuple structs with more than one field
  - Lists and arrays.
- Field annotations and more customization

(Note: A lot of work on serialization remains to be done. Because of the 'grouping' feature,
`bevy_basic_prefs` uses a custom conversion from Rust to TOML rather than relying on `serde`.
Currently, only a small number of Rust types are supported.)

## Non-goals

Because this library supports "basic" preferences, some things have been intentionally left out:

- Serialization of exotic types - we don't support serialization of every possible Rust type.
- Choice of config file formats.
- Hot loading / settings file change detection. Because the only program that ever writes to the
  settings file is the game itself, there's no need to be notified when the file has changed
  (and it would significantly complicate the design).

## Usage

### Install the Plugin

Install the preferences plugin in your app setup:

```rust
    // The argument is the name of the folder under which the settings will be saved.
    // So for example on Linux, this would result in something like
    // "~/.config/my_app_name/prefs.toml"
    app.add_plugins(PreferencesPlugin::new("my_app_name"));
```

### Annotate Resources

To load and save a resource as a preference, you must do two things (besides initializing it as a
resource):

- You must register it with the Bevy type registry.
- You must to annotate with either `PreferencesGroup`, `PreferencesKey`, or both.

- `PreferencesGroup(name)` indicates the name of the TOML table or group under which the
  item will appear.
- `PreferencesKey(name)` indicates the table key used to store the item. This defaults to
  the name of the field if not specified, unless it's a struct type, in which case each
  of the fields in the struct will have it's own key.

So for example:

```rust
#[derive(Resource, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("zoom"), @PreferencesKey("level"))]
pub struct ZoomLevel(pub f32);
```

This will produce a TOML file that has the following entry:

```toml
[zoom]
level = 0.0
```

### Annotate States

You can also use `PreferenceGroup` and `PreferenceKey` on Bevy game states, however there is one
difference: instead of registering the type of the state, you must register _both_ the
`State<MyState>` and `NextState<MyState>` types. During load, the library will automatically
update the `NextState` resource, causing a transition to the saved state.

### Loading

The plugin will automatically load all registered preference items in the App's `finish()` method,
which occurs after `init()` but before the `Startup` system runs.

### Saving

To automatically detect when preferences change and trigger a delayed save, add the following
to your app init:

```rust
app.add_systems(Update, watch_prefs_changes);
```

If you prefer not to do this, then you can manually trigger one of the following commands:

```rust
/// Marks prefs as changed, will save after second.
commands.add(SetPreferencesChanged);

/// Save preferences now.
commands.add(SavePreferences::Always);

/// Save preferences now, but only if they are changed.
commands.add(SavePreferences::IfChanged);
```
