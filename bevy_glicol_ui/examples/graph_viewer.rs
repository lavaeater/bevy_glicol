//! Run with:
//!   cargo run --example graph_viewer -p bevy_glicol_ui
//!
//! Loads `assets/glicols/test.glicol` on startup, shows the node graph on the
//! canvas and plays audio via `bevy_glicol`.  Click a node card to select it.
//! Press L to cycle through the bundled patches.

use bevy::prelude::*;
use bevy_glicol::GlicolPlugin;
use bevy_glicol_ui::{GlicolUiConfig, GlicolUiPlugin, LoadGlicolFile};

const PATCHES: &[&str] = &[
    "glicols/test.glicol",
    "glicols/test2.glicol",
    "glicols/synth.glicol",
    "glicols/SolsticeStream2023.glicol",
];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Glicol Graph Viewer".into(),
                resolution: (1400.0, 900.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GlicolPlugin)
        .add_plugins(GlicolUiPlugin)
        .insert_resource(PatchIndex(0))
        .add_systems(Startup, setup_camera)
        .add_systems(Update, cycle_patch)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Resource)]
struct PatchIndex(usize);

fn cycle_patch(
    keys: Res<ButtonInput<KeyCode>>,
    mut index: ResMut<PatchIndex>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        index.0 = (index.0 + 1) % PATCHES.len();
        commands.trigger(LoadGlicolFile(PATCHES[index.0].to_string()));
        info!("Loading patch: {}", PATCHES[index.0]);
    }
}
