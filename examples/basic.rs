use bevy::{color::palettes, math::vec2, prelude::*};
use bevy_ed2d::Ed2dPlugin;
use bevy_mod_picking::PickableBundle;

fn main() {
    App::new()
        .insert_resource(ClearColor(palettes::tailwind::NEUTRAL_800.into()))
        .add_plugins((DefaultPlugins, Ed2dPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Blue square"),
        SpriteBundle {
            sprite: Sprite {
                color: palettes::tailwind::BLUE_500.into(),
                custom_size: Some(vec2(100., 100.)),
                ..default()
            },
            ..default()
        },
        PickableBundle::default(),
    ));

    commands.spawn((
        Name::new("Red rectangle"),
        SpriteBundle {
            transform: Transform::from_translation(Vec3::new(0., -200., 0.)),
            sprite: Sprite {
                color: palettes::tailwind::RED_500.into(),
                custom_size: Some(vec2(300., 50.)),
                ..default()
            },
            ..default()
        },
        PickableBundle::default(),
    ));
}
