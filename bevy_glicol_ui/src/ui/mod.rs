pub mod graph_view;
pub mod node_card;

use bevy::prelude::*;
use lava_ui_builder::LavaUiPlugin;

use crate::asset::{
    GlicolSource, GlicolSourceLoader, LoadGlicolFile, PendingGlicolLoad, SaveGlicolFile,
    on_glicol_asset_loaded, on_load_glicol, on_save_glicol,
};
use crate::graph::{GlicolGraph, NodeRegistry};
use graph_view::{draw_connection_arrows, handle_card_clicks, rebuild_graph_cards, spawn_graph_canvas};

// ── Config ────────────────────────────────────────────────────────────────────

/// Optional config — insert before `GlicolUiPlugin` to customise startup.
#[derive(Resource)]
pub struct GlicolUiConfig {
    pub startup_file: Option<String>,
}

impl Default for GlicolUiConfig {
    fn default() -> Self {
        Self { startup_file: Some("glicols/test.glicol".into()) }
    }
}

// ── UI state ──────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct UiState {
    pub selected_node: Option<String>,
}

#[derive(Component)]
pub struct NodeListPanel;

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct GlicolUiPlugin;

impl Plugin for GlicolUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(LavaUiPlugin)
            .init_asset::<GlicolSource>()
            .init_asset_loader::<GlicolSourceLoader>()
            .init_resource::<NodeRegistry>()
            .init_resource::<GlicolGraph>()
            .init_resource::<UiState>()
            .init_resource::<PendingGlicolLoad>()
            .init_resource::<GlicolUiConfig>()
            .add_observer(on_load_glicol)
            .add_observer(on_save_glicol)
            .add_systems(Startup, (spawn_graph_canvas, spawn_side_panel, trigger_startup_load).chain())
            .add_systems(Update, (
                on_glicol_asset_loaded,
                sync_graph_to_engine,
                rebuild_graph_cards,
                handle_card_clicks,
                draw_connection_arrows,
            ));
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

fn trigger_startup_load(config: Res<GlicolUiConfig>, mut commands: Commands) {
    if let Some(path) = config.startup_file.clone() {
        commands.trigger(LoadGlicolFile(path));
    }
}

fn spawn_side_panel(mut commands: Commands) {
    commands.spawn((
        NodeListPanel,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(28.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(4.0),
            overflow: Overflow::clip_y(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.12, 0.92)),
    ));
}

// ── Graph → engine sync ───────────────────────────────────────────────────────

fn sync_graph_to_engine(
    graph: Res<GlicolGraph>,
    engine: Option<Res<bevy_glicol::prelude::GlicolEngine>>,
) {
    if !graph.is_changed() { return; }
    let code = graph.to_glicol_code();
    if code.is_empty() { return; }
    match engine {
        Some(e) => e.update_with_code(&code),
        None => info!("[glicol] (no engine) code:\n{}", code),
    }
}
