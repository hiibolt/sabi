use std::{ops::Index, time::Duration};
use anyhow::Context;
use bevy::prelude::*;
use crate::{
    VisualNovelState,
    actor::{
        CharacterConfig,
        controller::{
            ActorConfig, ActorPosition, ActorsResource, AnimationPosition, AnimationScale, AnimationTimer, CharacterDirection, CharacterPosition, FadingActors, MovingActors, SpawnInfo, SpriteIdentifier, SpriteKey
        }
    },
    compiler::controller::SabiState
};
use crate::compiler::controller::UiRoot;

const MOVEMENT_STEP: f32 = 0.4;
const CHARACTERS_Z_INDEX: i32 = 3;

#[derive(Component)]
pub struct Actor;

pub(in crate::actor) fn position_relative_to_center(
    (left, bottom): (f32, f32),
    (image_w, image_h): (usize, usize),
    scale: f32,
    window: &Window,
) -> (f32, f32) {
    let (w_pct, h_pct) = (image_w as f32 * scale / window.resolution.width() * 100., image_h as f32 * scale / window.resolution.height() * 100.);
    (
        left - w_pct / 2.,
        bottom - h_pct / 2.,
    )
}
pub fn change_character_emotion(
    image: &mut ImageNode,
    sprites: &Res<ActorsResource>,
    emotion: &str,
    config: &CharacterConfig
) -> Result<(), BevyError> {
   let sprite_key = SpriteKey {
       character: config.name.clone(),
       outfit: config.outfit.clone(),
       emotion: emotion.to_owned()
   };
   let sprite = sprites.0.get(&SpriteIdentifier::Character(sprite_key.clone())).context(format!("Sprite not found for {:?}", sprite_key))?;
   image.image = sprite.clone();
   
   Ok(())
}
pub fn move_characters(
    query: Query<(Entity, &mut Node), With<Actor>>,
    mut moving_actors: ResMut<MovingActors>,
    mut game_state: ResMut<VisualNovelState>,
) {
    for (entity, mut node) in query {
        let enumerated_element = moving_actors.0.iter().enumerate().find(|(_, e)| e.0 == entity);
        if let Some((index, target_pos)) = enumerated_element {
            let new_coords: (f32, f32) = match (node.left, node.bottom) {
                (Val::Percent(left), Val::Percent(btm)) => {
                    let new_left = if (left - target_pos.1.0).abs() < MOVEMENT_STEP {
                        target_pos.1.0
                    } else if left < target_pos.1.0 {
                        left + MOVEMENT_STEP
                    } else { left - MOVEMENT_STEP };
                    let new_bottom = if (btm - target_pos.1.1).abs() < MOVEMENT_STEP {
                        target_pos.1.1
                    } else if btm < target_pos.1.1 {
                        btm + MOVEMENT_STEP
                    } else { btm - MOVEMENT_STEP };
                    
                    (new_left, new_bottom)
                },
                _ => {
                    warn!("Movement directives accepts only actors with percentage value as position!");
                    moving_actors.0.remove(index);
                    if moving_actors.0.is_empty() {
                        game_state.blocking = false;
                        return;
                    }
                    continue;
                }
            };
            node.left = percent(new_coords.0);
            node.bottom = percent(new_coords.1);
            if new_coords == target_pos.1 {
                moving_actors.0.remove(index);
            }
            if moving_actors.0.is_empty() {
                game_state.blocking = false;
                return;
            }
        }
    }
}
pub fn apply_alpha(
    mut commands: Commands,
    mut query: Query<&mut ImageNode, With<Actor>>,
    mut fading_actors: ResMut<FadingActors>,
    mut game_state: ResMut<VisualNovelState>,
) {
    if fading_actors.0.is_empty() {
        return;
    }

    let mut finished_anim: Vec<Entity> = Vec::new();
    for actor in &fading_actors.0 {
        let mut s = match query.get_mut(actor.0) {
            Ok(e) => e,
            Err(_) => continue
        };
        let mut color = s.color;
        color.set_alpha(s.color.alpha() + actor.1);
        s.color = color;
        if color.alpha() >= 1. || color.alpha() <= 0. {
            finished_anim.push(actor.0);
        }
    }
    let mut to_remove: Vec<usize> = Vec::new();
    fading_actors.0.iter().enumerate().for_each(|f| {
        if finished_anim.contains(&f.1.0) {
            to_remove.push(f.0);
        }
    });
    to_remove.reverse();
    for index in to_remove {
        let item = fading_actors.0.index(index);
        let to_despawn = item.2;
        if to_despawn {
            commands.entity(item.0).despawn();
        }
        fading_actors.0.remove(index);
    }
    if fading_actors.0.is_empty() {
        game_state.blocking = false;
    }
}
pub fn spawn_actor(
    commands: &mut Commands,
    actor_config: ActorConfig,
    sprites: &Res<ActorsResource>,
    fading_actors: &mut ResMut<FadingActors>,
    ui_root: &Single<Entity, With<UiRoot>>,
    images: &Res<Assets<Image>>,
    info: SpawnInfo,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    window: &Window,
) -> Result<(), BevyError> {
    let actor_entity = match actor_config {
        ActorConfig::Character(actor_config) => {
            let sprite_key = SpriteKey {
                character: actor_config.name.clone(),
                outfit: actor_config.outfit.clone(),
                emotion: actor_config.emotion.clone(),
            };
            let image = sprites.0.get(&SpriteIdentifier::Character(sprite_key.clone())).context(format!("No sprite found for {:?}", sprite_key))?;
            let image_asset = images.get(image).context(format!("Asset not found for {:?}", image))?;
            let aspect_ratio = image_asset.texture_descriptor.size.width as f32 / image_asset.texture_descriptor.size.height as f32;
            let position = if let Some(pos) = info.position {
                match pos {
                    ActorPosition::Character(a) => a,
                    _ => { return Err(anyhow::anyhow!(format!("Expected Character position, found {:?}", pos)).into()); }
                }
            } else { CharacterPosition::default() };
            commands.spawn(
                (
                    ImageNode {
                        image: image.clone(),
                        color: Color::default().with_alpha(if info.fading {
                            0.
                        } else { 1. }),
                        flip_x: info.direction == CharacterDirection::Left,
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
                    Actor,
                    ActorConfig::Character(actor_config),
                    DespawnOnExit(SabiState::Running)
                )
            ).id()
        },
        ActorConfig::Animation(actor_config) => {
            let anim_id = actor_config.name.clone();
            let image = sprites.0.get(&SpriteIdentifier::Animation(anim_id.clone())).context(format!("No sprite found for {:?}", anim_id))?;
            let image_asset = images.get(image).context(format!("Asset not found for {:?}", image))?;
            let (image_width, image_height) = (image_asset.texture_descriptor.size.width as f32, image_asset.texture_descriptor.size.height as f32);
            let aspect_ratio = image_width / image_height;
            let layout = TextureAtlasLayout::from_grid(UVec2 {
                x: actor_config.width as u32,
                y: actor_config.height as u32
            }, actor_config.columns as u32, actor_config.rows as u32, None, None);
            let atlas_handle = texture_atlas_layouts.add(layout);
            let position = if let Some(pos) = info.position {
                match pos {
                    ActorPosition::Animation(a) => a,
                    _ => { return Err(anyhow::anyhow!(format!("Expected Animation position, found {:?}", pos)).into()); }
                }
            } else { AnimationPosition::default() };
            
            let scale = info.scale.unwrap_or(1.);
            if scale < 0. { return Err(anyhow::anyhow!("Scale value can´t be negative: {}", scale).into()); }
            let (left, bottom): (f32, f32) = position_relative_to_center(
                position.into(),
                (actor_config.width, actor_config.height),
                scale,
                window,
            );
            
            commands.spawn(
                (
                    ImageNode {
                        image: image.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: atlas_handle,
                            index: actor_config.start_index,
                        }),
                        color: Color::default().with_alpha(if info.fading {
                            0.
                        } else { 1. }),
                        flip_x: info.direction == CharacterDirection::Left,
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        aspect_ratio: Some(aspect_ratio),
                        width: px(actor_config.width as f32 * scale),
                        height: px(actor_config.height as f32 * scale),
                        left: percent(left),
                        bottom: percent(bottom),
                        ..default()
                    },
                    ZIndex(CHARACTERS_Z_INDEX),
                    Actor,
                    AnimationScale(scale),
                    AnimationTimer(Timer::new(Duration::from_secs_f32(1. / (actor_config.fps as f32)), TimerMode::Repeating)),
                    ActorConfig::Animation(actor_config),
                    DespawnOnExit(SabiState::Running)
                )
            ).id()
        }
    };
    commands.entity(ui_root.entity()).add_child(actor_entity);
    if info.fading {
        fading_actors.0.push((actor_entity, 0.01, false));
    }
    Ok(())
}
