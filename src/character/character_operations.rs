use std::ops::Index;
use anyhow::Context;
use bevy::prelude::*;
use crate::{VisualNovelState, character::{CharacterConfig, CharactersResource, controller::{CharacterPosition, FadingActors, MovingActors, SpriteKey}}, compiler::controller::SabiState};
use crate::compiler::controller::UiRoot;

const MOVEMENT_STEP: f32 = 0.4;
const CHARACTERS_Z_INDEX: i32 = 3;

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
pub fn move_characters(
    query: Query<(Entity, &mut Node), With<Character>>,
    mut moving_characters: ResMut<MovingActors>,
    mut game_state: ResMut<VisualNovelState>,
) {
    for (entity, mut node) in query {
        let enumerated_element = moving_characters.0.iter().enumerate().find(|(_, e)| e.0 == entity);
        if let Some((index, target_pos)) = enumerated_element {
            let new_value = match node.left {
                Val::Percent(val) => {
                    if (val - target_pos.1).abs() < MOVEMENT_STEP {
                        target_pos.1
                    } else if val < target_pos.1 {
                        val + MOVEMENT_STEP
                    } else { val - MOVEMENT_STEP }
                },
                _ => {
                    warn!("Movement directives accepts only characters with percentage value as position!");
                    moving_characters.0.remove(index);
                    if moving_characters.0.is_empty() {
                        game_state.blocking = false;
                        return;
                    }
                    continue;
                }
            };
            node.left = percent(new_value);
            if new_value == target_pos.1 {
                moving_characters.0.remove(index);
            }
            if moving_characters.0.is_empty() {
                game_state.blocking = false;
                return;
            }
        }
    }
}
pub fn apply_alpha(
    mut commands: Commands,
    mut query: Query<&mut ImageNode, With<Character>>,
    mut fading_characters: ResMut<FadingActors>,
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
    fading: bool,
    fading_characters: &mut ResMut<FadingActors>,
    ui_root: &Single<Entity, With<UiRoot>>,
    images: &Res<Assets<Image>>,
    position: CharacterPosition,
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
                color: Color::default().with_alpha(if fading {
                    0.
                } else { 1. }),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                max_height: percent(75.),
                bottom: percent(0.),
                aspect_ratio: Some(aspect_ratio),
                left: percent(position.to_percentage_value()),
                ..default()
            },
            ZIndex(CHARACTERS_Z_INDEX),
            Character,
            character_config,
            DespawnOnExit(SabiState::Running)
        )
    ).id();
    commands.entity(ui_root.entity()).add_child(character_entity);
    if fading {
        fading_characters.0.push((character_entity, 0.01, false));
    }
    Ok(())
}
