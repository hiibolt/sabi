use std::ops::Index;
use anyhow::Context;
use bevy::prelude::*;
use crate::{character::{controller::{FadingCharacters, SpriteKey}, CharacterConfig, CharactersResource}, VisualNovelState};
use crate::compiler::controller::UiRoot;

#[derive(Component)]
pub struct Character;

pub fn change_character_emotion(
    image: &mut ImageNode,
    sprites: &Res<CharactersResource>,
    emotion: &str,
    config: &CharacterConfig
) -> Result<(), BevyError> {
   let sprite_key = SpriteKey {
       character: config.name.clone(),
       outfit: config.outfit.clone(),
       emotion: emotion.to_owned()
   };
   let sprite = sprites.0.get(&sprite_key).with_context(|| format!("Sprite not found for {:?}", sprite_key))?;
   image.image = sprite.clone();
   
   Ok(())
}
pub fn apply_alpha(
    mut commands: Commands,
    mut query: Query<&mut ImageNode, With<Character>>,
    mut fading_characters: ResMut<FadingCharacters>,
    mut game_state: ResMut<VisualNovelState>,
) {
    if fading_characters.0.is_empty() {
        return;
    }

    let mut finished_anim: Vec<Entity> = Vec::new();
    for fading_char in &fading_characters.0 {
        let mut s = match query.get_mut(fading_char.0) {
            Ok(e) => e,
            Err(_) => continue
        };
        let mut color = s.color;
        color.set_alpha(s.color.alpha() + fading_char.1);
        s.color = color;
        if color.alpha() >= 1. || color.alpha() <= 0. {
            finished_anim.push(fading_char.0);
        }
    }
    let mut to_remove: Vec<usize> = Vec::new();
    fading_characters.0.iter().enumerate().for_each(|f| {
        if finished_anim.contains(&f.1.0) {
            to_remove.push(f.0);
        }
    });
    to_remove.reverse();
    for index in to_remove {
        let item = fading_characters.0.index(index);
        let to_despawn = item.2;
        if to_despawn {
            commands.entity(item.0).despawn();
        }
        fading_characters.0.remove(index);
    }
    if fading_characters.0.is_empty() {
        game_state.blocking = false;
    }
}
pub fn spawn_character(
    commands: &mut Commands,
    character_config: CharacterConfig,
    sprites: &Res<CharactersResource>,
    fading: &bool,
    fading_characters: &mut ResMut<FadingCharacters>,
    ui_root: &Single<Entity, With<UiRoot>>,
    images: &Res<Assets<Image>>,
) -> Result<(), BevyError> {
    let sprite_key = SpriteKey {
        character: character_config.name.clone(),
        outfit: character_config.outfit.clone(),
        emotion: character_config.emotion.clone(),
    };
    let image = sprites.0.get(&sprite_key).with_context(|| format!("No sprite found for {:?}", sprite_key))?;
    let image_asset = images.get(image).with_context(|| format!("Asset not found for {:?}", image))?;
    let aspect_ratio = image_asset.texture_descriptor.size.width as f32 / image_asset.texture_descriptor.size.height as f32;
    let character_entity = commands.spawn(
        (
            ImageNode {
                image: image.clone(),
                color: Color::default().with_alpha(if *fading {
                    0.
                } else { 1. }),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                max_height: Val::Vh(75.),
                bottom: Val::Px(0.),
                aspect_ratio: Some(aspect_ratio),
                ..default()
            },
            ZIndex(2),
            Character,
            character_config
        )
    ).id();
    commands.entity(ui_root.entity()).add_child(character_entity);
    if *fading {
        fading_characters.0.push((character_entity, 0.01, false));
    }
    Ok(())
}
