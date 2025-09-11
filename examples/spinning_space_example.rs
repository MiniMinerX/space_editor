use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
};
use space_editor::prelude::*;

use space_editor_ui::ext::bevy_panorbit_camera::PanOrbitCamera;
use transform_gizmo_bevy::{GizmoCamera, GizmoTarget, TransformGizmoPlugin};

fn main() {
    App::default()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            level: Level::DEBUG,
            ..default()
        }))
        .add_plugins(SpaceEditorPlugin)
        .add_systems(Startup, simple_editor_setup)

        .register_type::<GizmoCamera>()
        .register_type::<GizmoTarget>()
        .editor_registry::<GizmoCamera>()
        .editor_registry::<GizmoTarget>()
        .register_type::<PanOrbitCamera>()


        .add_systems(Update, spin_entities)
        .register_type::<SpinningMarker>()
        .editor_registry::<SpinningMarker>()

        .run();
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct SpinningMarker {
    active: bool,
    speed: f32,
    axis: Vec3,
}

impl Default for SpinningMarker {
    fn default() -> Self {
        Self {
            active: false,
            speed: std::f32::consts::PI / 2.0, // 90 degrees per second
            axis: Vec3::Y,
        }
    }
}

fn spin_entities(
    mut spinning_entities: Query<(&SpinningMarker, &mut Transform)>,
    time: Res<Time>,
) {
    for (marker, mut transform) in spinning_entities.iter_mut() {
        if marker.active {
            let normalized_axis = if marker.axis.length_squared() == 0.0 {
                Vec3::Y
            } else {
                marker.axis.normalize()
            };
            transform.rotate(Quat::from_axis_angle(
                normalized_axis,
                marker.speed * time.delta_secs(),
            ));
        }
    }
}