# Sabi

Sabi is a visual novel engine built on Rust and Bevy ECS. It provides a domain-specific scripting language with stage direction syntax, a modular character system with emotion-based sprites, and runtime asset management through Bevy's asset pipeline.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/bevy-2C2D33?style=for-the-badge&logo=bevy&logoColor=white)

## Features

**Scripting Language**
- Pest-based parser with stage direction syntax inspired by theatrical scripts
- Scene-based organization with named transitions
- Expression evaluation supporting string concatenation and variable substitution
- Inline emotion changes during dialogue

**Actor System**
- JSON-based character definitions with sprite mappings per emotion and outfit
- Frame-based sprite sheet animations with configurable FPS
- Character positioning system using percentage-based coordinates that scale with window size
- Fade in/out and directional facing with automatic sprite flipping
- Actor movement with interpolation between positions

**Rendering**
- ECS-based architecture separating character state from visual representation
- Background management with transition support
- Customizable GUI elements (textbox, namebox) with 9-slice and auto scaling
- Text rendering with character-by-character reveal animation
- History system tracking all dialogue and stage directions

**Development Environment**
- Nix flake for reproducible builds
- Hot-reloadable assets during development
- Six example projects demonstrating different features
- Modular plugin architecture for extending functionality

## Getting Started

### Prerequisites
- Rust toolchain (1.80+)
- Cargo

### Running Examples

```bash
cargo run --example basic_startup
cargo run --example character_operations
cargo run --example animation
cargo run --example background
cargo run --example ui
cargo run --example infotext
```

### Using Nix
```bash
nix develop
cargo run --example basic_startup
```

## Script Language

Sabi scripts use a `.sabi` extension and organize content into scenes with stage directions:

```
SCENE opening
    (GUI textbox changes to "TEXTBOX_DEFAULT")
    (Background changes to "classroom_day")
    (Nayu appears center)
    Nayu: "This is dialogue."
    Nayu: (happy) "Emotions can change inline."
    MC: "Player character speaks like this."
    (Nayu moves right)
    (Nayu looks left)
    (Nayu fade out)
    (Scene "next_scene" begins)
CURTAIN
```

### Supported Commands

**Character Operations**
- `(Character appears [position] [looking direction] [emotion])`
- `(Character disappears)`
- `(Character fade in/out)`
- `(Character moves position)`
- `(Character looks left/right)`

**Positions**: `center`, `left`, `right`, `far left`, `far right`, `invisible left`, `invisible right`

**Scene Management**
- `(Scene "scene_name" begins)` - Jump to different scene
- `(Background changes to "background_id")`
- `(GUI element changes to "sprite_id" [sliced|auto])` - Customize textbox/namebox

**Animations**
- `(Animated "animation_id" appears position scale N)`
- `(Animated "animation_id" fade in/out)`
- `(Animated "animation_id" moves position)`
- `(Animated "animation_id" looks left/right)`

**Dialogue**
- `Character: "dialogue text"` - Named character speaks
- `Character: (emotion) "dialogue"` - Inline emotion change
- `MC: "dialogue"` - Main character (substitutes player name)
- `info: "text"` - Narrator/info text

**Logging**
- `{log "debug message"}` - Console output for development

## Project Structure

```
src/
├── actor/           # Character and animation controllers
├── background/      # Background rendering system
├── chat/            # Dialogue box and text animation
├── compiler/        # Script parser and AST
└── loader/          # Asset loaders for JSON and .sabi files

assets/sabi/
├── acts/            # Script files organized by chapter
├── animations/      # Animation sprite sheets (JSON)
├── backgrounds/     # Background images
├── characters/      # Character sprites and configs
├── fonts/           # Text rendering fonts
└── ui/              # UI element sprites
```

## Integration

Add Sabi to your Bevy app:

```rust
use sabi::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SabiPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut msg_writer: MessageWriter<SabiStart>,
    mut user_defined_constants: ResMut<UserDefinedConstants>,
) {
    user_defined_constants.playername = "Player".into();
    commands.spawn(Camera2d::default());
    msg_writer.write(SabiStart(ScriptId { 
        chapter: "chapter1".into(), 
        act: "opening".into() 
    }));
}
```

## Architecture

Sabi uses Bevy's ECS with a plugin-based architecture:

- **Compiler Plugin**: Parses `.sabi` scripts into an AST, evaluates expressions, manages scene transitions and game state
- **Character Controller**: Spawns/despawns actors, handles movement interpolation, manages fade effects and sprite switching
- **Chat Controller**: Renders dialogue boxes, implements text reveal animation, maintains conversation history
- **Background Controller**: Loads and transitions between background images

The system uses Bevy's message passing for coordination between plugins and resource-based state management for the script execution cursor and character configurations.

## Asset Configuration

**Character Definition** (`character.json`):
```json
{
  "name": "Nayu",
  "outfits": {
    "uniform": {
      "emotions": {
        "neutral": "nayu_neutral.png",
        "happy": "nayu_happy.png",
        "concerned": "nayu_concerned.png"
      }
    }
  }
}
```

**Animation Definition** (`animation.json`):
```json
{
  "name": "fire",
  "spritesheet": "fire_sheet.png",
  "frames": 8,
  "fps": 12,
  "frame_width": 64,
  "frame_height": 64
}
```

## Current Limitations

- No save/load system
- Audio not implemented
- No branching/choice system
- Text input requires external implementation
- Asset paths are hardcoded relative to `assets/sabi/`

## License

See [LICENSE.md](LICENSE.md)

## Credits

- UI Panels by [BDragon1727](https://bdragon1727.itch.io/custom-border-and-panels-menu-all-part)
- Fire animation by [Devkidd](https://devkidd.itch.io/pixel-fire-asset-pack)