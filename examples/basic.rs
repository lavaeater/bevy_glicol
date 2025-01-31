use bevy::prelude::*;
use bevy_glicol::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GlicolPlugin))
        .insert_resource(Vol(0.5))
        .add_systems(Update, play_tone)
        .run();
}

#[derive(Resource)]
pub struct Vol(f32);


fn play_tone(engine: Res<GlicolEngine>, mut vol: ResMut<Vol>, input: Res<ButtonInput<KeyCode>>) {
    let codes = "out: sin ~pitch >> mul ~envb >> mul 0.9

~envb: ~triggerb >> envperc 0.01 0.4;

~env_pitch: ~triggerb >> envperc 0.01 0.1;

~pitch: ~env_pitch >> mul 50 >> add 60;

~triggerb: speed 4.0 >> seq 60";
    
    if input.pressed(KeyCode::ArrowUp) {
        vol.0 += 0.01;
        vol.0 = vol.0.min(1.0);
    } else if input.pressed(KeyCode::ArrowDown) {
        vol.0 -= 0.01;
        vol.0 = vol.0.max(0.0);
    }
    // engine.update_with_code(&format!("o: sin {}", vol.0 * 440.0 + 220.0));
    engine.update_with_code(codes);
}
