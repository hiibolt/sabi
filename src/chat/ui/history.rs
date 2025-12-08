use bevy::color::palettes::css::GRAY;
use bevy::{asset::AssetServer, ecs::relationship::RelatedSpawner};
use bevy::prelude::*;
use bevy_ui_widgets::{CoreScrollbarThumb, Scrollbar};

use crate::chat::controller::{HistoryScrollbar, HistoryText, UiButtons};
use crate::chat::ui::basic::button;
use crate::{VisualNovelState, chat::{UI_Z_INDEX, controller::{CurrentTextBoxBackground, HistoryPanel}}};

pub(crate) fn history_panel(
    current_plate: Res<CurrentTextBoxBackground>,
    game_state: &ResMut<VisualNovelState>,
    asset_server: &Res<AssetServer>,
) -> Result<impl Bundle, BevyError> {
    
    let history_text = history_text(asset_server, game_state)?;
    let exit_history_button = button(UiButtons::ExitHistory)?;
    
    Ok((
        ImageNode {
            image: current_plate.0.image.clone(),
            image_mode: current_plate.0.image_mode.clone(),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            width: percent(70.),
            height: percent(65.),
            top: percent(3.),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            padding: UiRect {
                top: percent(6.),
                bottom: percent(2.),
                ..UiRect::horizontal(percent(4.))
            },
            ..default()
        },
        ZIndex(UI_Z_INDEX),
        HistoryPanel,
        Children::spawn(
            SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                parent.spawn(history_title());
                let scroll_area_id = parent.spawn((
                    history_text,
                )).id();
                parent.spawn(scrollbar(scroll_area_id));
                parent.spawn(exit_history_button);
            })
        ),
    ))
}

fn history_title() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: percent(3.),
            ..default()
        },
        Text::new("History"),
        TextFont {
            font_size: 21.,
            ..default()
        }
    )
}

fn scrollbar(entity: Entity) -> impl Bundle {
    (
        Node {
            min_width: px(8.),
            ..default()
        },
        Scrollbar {
            orientation: bevy_ui_widgets::ControlOrientation::Vertical,
            target: entity,
            min_thumb_length: 8.,
        },
        HistoryScrollbar,
        children![
            (
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(GRAY.into()),
                BorderRadius::all(px(4.)),
                CoreScrollbarThumb,
            )
        ]
    )
}

fn history_text(asset_server: &Res<AssetServer>, game_state: &ResMut<VisualNovelState>) -> Result<impl Bundle, BevyError> {
    let history_text = game_state.history_summary()?.join("\n");
    let font_handle = asset_server.load("fonts/ALLER.ttf");
    Ok((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(100.),
            height: percent(100.),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.,
            ..default()
        },
        children![
            (
                Text(history_text),
                TextFont {
                    font: font_handle,
                    font_size: 14.,
                    ..default()
                },
            )
        ],
        ZIndex(UI_Z_INDEX),
        ScrollPosition(Vec2::new(0., 0.)),
        HistoryText
    ))
}