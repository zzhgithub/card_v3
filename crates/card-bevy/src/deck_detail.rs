use crate::app_state::{
    AddCardButton, AppState, AvailableCardItem, AvailableCards, DeckCardItem,
    DeckDetailReturnButton, LeftPanel, MiddlePanel, RemoveCardButton, SaveDeckButton, SelectedCard,
    SelectedDeck,
};
use crate::colors::*;
use crate::ui_components::{spawn_status_bar, spawn_version_display};
use bevy::prelude::*;
use card_core::deck::{Deck, DeckManager};
use card_core::effect::text::card_effects_text;
use card_core::types::{CardDefinition, CardId, CardType, ItemKind, StrategyKind};
use card_script::loader::{ScriptIndex, ScriptLoader};
use std::path::Path;

pub fn load_available_cards(mut available_cards: ResMut<AvailableCards>) {
    let scripts_dir = Path::new("scripts");
    if let Ok(index) = ScriptIndex::scan(scripts_dir) {
        let loader = ScriptLoader::new(index);
        if let Ok(registry) = loader.load_all() {
            available_cards.registry = registry;
        }
    }
}

pub fn enter_deck_detail(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected_deck: Res<SelectedDeck>,
    available_cards: Res<AvailableCards>,
) {
    spawn_version_display(&mut commands, &asset_server);
    spawn_status_bar(
        &mut commands,
        &asset_server,
        &format!("卡组详情 - {}", selected_deck.name),
    );

    let font = asset_server.load(FONT_PATH);

    spawn_return_button(&mut commands, font.clone());
    spawn_save_button(&mut commands, font.clone());

    let main_container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(60.0), Val::Px(20.0)),
            ..default()
        })
        .id();

    commands.entity(main_container).with_children(|parent| {
        spawn_left_panel(parent, &asset_server, None);
        spawn_middle_panel(parent, &asset_server, &selected_deck);
        spawn_right_panel(parent, &asset_server, &available_cards);
    });
}

fn spawn_return_button(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Button,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(40.0),
                position_type: PositionType::Absolute,
                right: Val::Px(140.0),
                top: Val::Px(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COLOR_BUTTON),
            DeckDetailReturnButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("返回"),
                TextFont {
                    font,
                    font_size: 18.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_save_button(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Button,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(40.0),
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.8, 0.3)),
            SaveDeckButton,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("保存"),
                TextFont {
                    font,
                    font_size: 18.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn spawn_left_panel(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    card: Option<&CardDefinition>,
) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::right(Val::Px(20.0)),
                ..default()
            },
            LeftPanel,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(400.0),
                        margin: UiRect::bottom(Val::Px(20.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                ))
                .with_children(|parent| {
                    let text = match card {
                        Some(c) => c.name.clone(),
                        None => "选择卡片查看详情".to_string(),
                    };
                    parent.spawn((
                        Text::new(text),
                        TextFont {
                            font: font.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                });

            if let Some(card) = card {
                spawn_card_details(parent, &font, card);
            }
        });
}

fn spawn_card_details(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    card: &CardDefinition,
) {
    let card_type_text = match card.card_type {
        CardType::Character => {
            let atk = card
                .attack
                .map_or("".to_string(), |a| format!(" (ATK: {})", a));
            format!("人物卡{}", atk)
        }
        CardType::Strategy => {
            let kind = card.strategy_kind.map_or("".to_string(), |k| match k {
                StrategyKind::Normal => " [普通]".to_string(),
                StrategyKind::Trick => " [诡计]".to_string(),
                StrategyKind::Instant => " [瞬时]".to_string(),
            });
            format!("策略卡{}", kind)
        }
        CardType::Item => {
            let kind = card.item_kind.map_or("".to_string(), |k| match k {
                ItemKind::Normal => " [普通]".to_string(),
                ItemKind::Persistent => " [存留]".to_string(),
            });
            format!("物品卡{}", kind)
        }
        CardType::Legendary => "传奇卡".to_string(),
    };

    let property_text = format!("{:?}", card.property);
    let category_text = format!("{:?}", card.category);

    let details = vec![
        format!("名称: {}", card.name),
        format!("ID: {}", card.id.0),
        format!("类型: {}", card_type_text),
        format!("属性: {}", property_text),
        format!("类别: {}", category_text),
        format!("费用: {}", card.cost),
    ];

    for detail in details {
        parent.spawn((
            Text::new(detail),
            TextFont {
                font: font.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(COLOR_TEXT),
        ));
        parent.spawn(Node {
            height: Val::Px(5.0),
            ..default()
        });
    }

    if !card.tags.is_empty() {
        parent.spawn((
            Text::new(format!("标签: {}", card.tags.join(", "))),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(COLOR_TEXT_DIM),
        ));
        parent.spawn(Node {
            height: Val::Px(10.0),
            ..default()
        });
    }

    if !card.effects.is_empty() {
        parent.spawn((
            Text::new("效果:"),
            TextFont {
                font: font.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(COLOR_TEXT),
        ));

        let effect_texts = card_effects_text(&card.effects);
        for (i, effect_text) in effect_texts.iter().enumerate() {
            parent.spawn((
                Text::new(format!("{}. {}", i + 1, effect_text)),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_DIM),
            ));
        }
    }
}

fn spawn_middle_panel(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    selected_deck: &SelectedDeck,
) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn(Node {
            width: Val::Px(350.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::right(Val::Px(20.0)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("卡组 ({}张)", selected_deck.cards.len())),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));

            parent.spawn(Node {
                height: Val::Px(10.0),
                ..default()
            });

            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    MiddlePanel,
                ))
                .with_children(|parent| {
                    if selected_deck.cards.is_empty() {
                        parent.spawn((
                            Text::new("卡组为空，从右侧添加卡片"),
                            TextFont {
                                font: font.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_DIM),
                        ));
                    } else {
                        for (index, card_id_str) in selected_deck.cards.iter().enumerate() {
                            spawn_deck_card_item(parent, &font, card_id_str, index);
                        }
                    }
                });
        });
}

fn spawn_deck_card_item(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    card_id_str: &str,
    index: usize,
) {
    if let Ok(card_id) = card_id_str.parse::<CardId>() {
        parent
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    margin: UiRect::vertical(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.25, 0.3, 1.0)),
                DeckCardItem {
                    card_id: card_id.clone(),
                    index,
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(card_id_str),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(COLOR_TEXT),
                ));

                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(25.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(COLOR_BUTTON_EXIT),
                        RemoveCardButton { index },
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new("×"),
                            TextFont {
                                font: font.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT),
                        ));
                    });
            });
    }
}

fn spawn_right_panel(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    available_cards: &AvailableCards,
) {
    let font = asset_server.load(FONT_PATH);

    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("可用卡片 ({}张)", available_cards.registry.len())),
                TextFont {
                    font: font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));

            parent.spawn(Node {
                height: Val::Px(10.0),
                ..default()
            });

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                })
                .with_children(|parent| {
                    let mut cards: Vec<_> = available_cards.registry.iter().collect();
                    cards.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));

                    for (card_id, card_def) in cards {
                        spawn_available_card_item(parent, &font, card_id, card_def);
                    }
                });
        });
}

fn spawn_available_card_item(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    card_id: &CardId,
    card_def: &CardDefinition,
) {
    let bg_color = get_card_type_color(&card_def.card_type);

    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(10.0)),
                margin: UiRect::vertical(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            AvailableCardItem {
                card_id: card_id.clone(),
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(card_def.name.clone()),
                        TextFont {
                            font: font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                    ));
                    parent.spawn((
                        Text::new(format!("{} (费用: {})", card_id.0, card_def.cost)),
                        TextFont {
                            font: font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(COLOR_BUTTON),
                    AddCardButton {
                        card_id: card_id.clone(),
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("+"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                    ));
                });
        });
}

fn get_card_type_color(card_type: &CardType) -> Color {
    match card_type {
        CardType::Character => COLOR_CARD_CHARACTER,
        CardType::Strategy => COLOR_CARD_STRATEGY,
        CardType::Item => COLOR_CARD_ITEM,
        CardType::Legendary => COLOR_CARD_LEGENDARY,
    }
}

fn get_card_type_color_hover(card_type: &CardType) -> Color {
    match card_type {
        CardType::Character => COLOR_CARD_CHARACTER_HOVER,
        CardType::Strategy => COLOR_CARD_STRATEGY_HOVER,
        CardType::Item => COLOR_CARD_ITEM_HOVER,
        CardType::Legendary => COLOR_CARD_LEGENDARY_HOVER,
    }
}

pub fn handle_deck_detail_return_button(
    mut interaction_query: Query<
        &Interaction,
        (Changed<Interaction>, With<DeckDetailReturnButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::DeckEditor);
            return;
        }
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::DeckEditor);
    }
}

pub fn handle_save_deck_button(
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<SaveDeckButton>)>,
    selected_deck: Res<SelectedDeck>,
) {
    for interaction in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            let card_ids: Vec<CardId> = selected_deck
                .cards
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let mut deck = Deck::new(&selected_deck.name);
            deck.cards = card_ids;
            if let Err(e) = DeckManager::save(Path::new(&selected_deck.file_path), &deck) {
                eprintln!("Failed to save deck: {}", e);
            } else {
                println!("Deck saved successfully: {}", selected_deck.name);
            }
        }
    }
}

pub fn handle_add_card_button(
    mut interaction_query: Query<(&Interaction, &AddCardButton), Changed<Interaction>>,
    mut selected_deck: ResMut<SelectedDeck>,
) {
    for (interaction, add_button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            selected_deck.cards.push(add_button.card_id.0.clone());
        }
    }
}

pub fn handle_remove_card_button(
    mut interaction_query: Query<(&Interaction, &RemoveCardButton), Changed<Interaction>>,
    mut selected_deck: ResMut<SelectedDeck>,
) {
    for (interaction, remove_button) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            if remove_button.index < selected_deck.cards.len() {
                selected_deck.cards.remove(remove_button.index);
            }
        }
    }
}

pub fn handle_deck_card_click(
    mut interaction_query: Query<(&Interaction, &DeckCardItem), Changed<Interaction>>,
    mut selected_card: ResMut<SelectedCard>,
) {
    for (interaction, deck_card) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            selected_card.id = Some(deck_card.card_id.clone());
        }
    }
}

pub fn handle_available_card_click(
    mut interaction_query: Query<(&Interaction, &AvailableCardItem), Changed<Interaction>>,
    mut selected_card: ResMut<SelectedCard>,
) {
    for (interaction, available_card) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            selected_card.id = Some(available_card.card_id.clone());
        }
    }
}

pub fn handle_available_card_hover(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &AvailableCardItem),
        Changed<Interaction>,
    >,
    available_cards: Res<AvailableCards>,
) {
    for (interaction, mut color, item) in &mut interaction_query {
        let card_type = available_cards
            .registry
            .get(&item.card_id)
            .map(|c| c.card_type);

        match *interaction {
            Interaction::Hovered => {
                *color = BackgroundColor(match card_type {
                    Some(CardType::Character) => COLOR_CARD_CHARACTER_HOVER,
                    Some(CardType::Strategy) => COLOR_CARD_STRATEGY_HOVER,
                    Some(CardType::Item) => COLOR_CARD_ITEM_HOVER,
                    Some(CardType::Legendary) => COLOR_CARD_LEGENDARY_HOVER,
                    None => COLOR_BUTTON_HOVER,
                });
            }
            Interaction::None => {
                *color = BackgroundColor(match card_type {
                    Some(CardType::Character) => COLOR_CARD_CHARACTER,
                    Some(CardType::Strategy) => COLOR_CARD_STRATEGY,
                    Some(CardType::Item) => COLOR_CARD_ITEM,
                    Some(CardType::Legendary) => COLOR_CARD_LEGENDARY,
                    None => COLOR_BUTTON,
                });
            }
            _ => {}
        }
    }
}

pub fn cleanup_deck_detail(
    mut commands: Commands,
    query: Query<Entity, With<Node>>,
    version_query: Query<Entity, With<crate::app_state::VersionText>>,
    status_query: Query<Entity, With<crate::app_state::StatusBarText>>,
) {
    for entity in query.iter() {
        if !version_query.get(entity).is_ok() && !status_query.get(entity).is_ok() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn rebuild_middle_panel(
    mut commands: Commands,
    selected_deck: Res<SelectedDeck>,
    asset_server: Res<AssetServer>,
    container_query: Query<(Entity, Option<&Children>), With<MiddlePanel>>,
) {
    if selected_deck.is_changed() {
        let font = asset_server.load(FONT_PATH);

        for (entity, children) in container_query.iter() {
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            commands.entity(entity).with_children(|parent| {
                if selected_deck.cards.is_empty() {
                    parent.spawn((
                        Text::new("卡组为空，从右侧添加卡片"),
                        TextFont {
                            font: font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT_DIM),
                    ));
                } else {
                    for (index, card_id_str) in selected_deck.cards.iter().enumerate() {
                        spawn_deck_card_item(parent, &font, card_id_str, index);
                    }
                }
            });
        }
    }
}

pub fn update_left_panel(
    mut commands: Commands,
    selected_card: Res<SelectedCard>,
    available_cards: Res<AvailableCards>,
    asset_server: Res<AssetServer>,
    left_panel_query: Query<(Entity, Option<&Children>), With<LeftPanel>>,
) {
    if selected_card.is_changed() {
        let font = asset_server.load(FONT_PATH);

        let card_def = selected_card
            .id
            .as_ref()
            .and_then(|id| available_cards.registry.get(id));

        for (entity, children) in left_panel_query.iter() {
            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            commands.entity(entity).with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            width: Val::Px(280.0),
                            height: Val::Px(400.0),
                            margin: UiRect::bottom(Val::Px(20.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 1.0)),
                    ))
                    .with_children(|parent| {
                        let text = match card_def {
                            Some(c) => format!("{}\n费用: {}", c.name, c.cost),
                            None => "选择卡片查看详情".to_string(),
                        };
                        parent.spawn((
                            Text::new(text),
                            TextFont {
                                font: font.clone(),
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_DIM),
                        ));
                    });

                if let Some(card) = card_def {
                    spawn_card_details(parent, &font, card);
                }
            });
        }
    }
}
