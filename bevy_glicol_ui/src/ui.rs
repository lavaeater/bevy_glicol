use bevy::prelude::*;
use lava_ui_builder::LavaUiPlugin;

use crate::asset::{
    GlicolSource, GlicolSourceLoader, LoadGlicolFile, PendingGlicolLoad, SaveGlicolFile,
    handle_load_event, handle_save_event, on_glicol_loaded,
};
use crate::graph::{GlicolGraph, NodeRegistry};

/// Configuration inserted before `GlicolUiPlugin` to control startup behaviour.
#[derive(Resource)]
pub struct GlicolUiConfig {
    /// If set, this `.glicol` file (relative to `assets/`) is loaded on startup.
    pub startup_file: Option<String>,
}

impl Default for GlicolUiConfig {
    fn default() -> Self {
        Self { startup_file: Some("glicols/test.glicol".into()) }
    }
}

/// Add this plugin to your Bevy app to get the Glicol UI and asset pipeline.
///
/// Also add `bevy_glicol::GlicolPlugin` if you want audio output.
pub struct GlicolUiPlugin;

impl Plugin for GlicolUiPlugin {
    fn build(&self, app: &mut App) {
        app
            // Sub-plugins
            .add_plugins(LavaUiPlugin)
            // Asset types
            .init_asset::<GlicolSource>()
            .init_asset_loader::<GlicolSourceLoader>()
            // Resources
            .init_resource::<NodeRegistry>()
            .init_resource::<GlicolGraph>()
            .init_resource::<UiState>()
            .init_resource::<PendingGlicolLoad>()
            .init_resource::<GlicolUiConfig>()
            // Events
            .add_event::<LoadGlicolFile>()
            .add_event::<SaveGlicolFile>()
            // Systems
            .add_systems(Startup, (spawn_ui, trigger_startup_load))
            .add_systems(Update, (
                handle_load_event,
                on_glicol_loaded,
                handle_save_event,
                sync_graph_to_engine,
            ));
    }
}

// ── UI state ──────────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct UiState {
    pub selected_node: Option<String>,
}

// ── Marker components ─────────────────────────────────────────────────────────

#[derive(Component)]
pub struct NodeListPanel;

#[derive(Component)]
pub struct NodeDetailPanel;

// ── Startup systems ───────────────────────────────────────────────────────────

fn trigger_startup_load(
    config: Res<GlicolUiConfig>,
    mut events: EventWriter<LoadGlicolFile>,
) {
    if let Some(path) = &config.startup_file {
        events.write(LoadGlicolFile(path.clone()));
    }
}

fn spawn_ui(mut commands: Commands) {
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
        BackgroundColor(Color::srgba(0.05, 0.05, 0.12, 0.90)),
    ));
}

// ── Graph → engine sync ───────────────────────────────────────────────────────

fn sync_graph_to_engine(
    graph: Res<GlicolGraph>,
    engine: Option<Res<bevy_glicol::GlicolEngine>>,
) {
    if !graph.is_changed() { return; }
    let code = graph.to_glicol_code();
    if code.is_empty() { return; }
    if let Some(engine) = engine {
        engine.update_with_code(&code);
    } else {
        info!("[glicol] (no engine) code:\n{}", code);
    }
}
