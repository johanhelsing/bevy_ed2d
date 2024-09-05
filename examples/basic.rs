use bevy::{math::vec2, prelude::*};
use bevy_ed2d::Ed2dPlugin;
use bevy_mod_picking::PickableBundle;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, Ed2dPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Debugging sprite
    commands.spawn((
        Name::new("Box"),
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgb(0.3, 0.3, 0.4),
                custom_size: Some(vec2(100.0, 100.0)),
                ..default()
            },
            ..default()
        },
        PickableBundle::default(),
    ));
}
