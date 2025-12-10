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
        .add_systems(Startup, spawn_test_hierarchy)

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
            speed: std::f32::consts::PI / 2.0,
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

fn spawn_test_hierarchy(mut commands: Commands) {
    // Spawn 10 parents, each with 100 children
    // 5 parents WITH PrefabMarker
    for parent_idx in 0..5 {
        let parent = commands.spawn((
            Name::new(format!("Parent WITH Prefab #{} (100 children)", parent_idx)),
            PrefabMarker,
            Transform::from_xyz(parent_idx as f32 * 3.0, 0.0, 0.0),
        )).id();

        for child_idx in 0..100 {
            let child = commands.spawn((
                Name::new(format!("Prefab Parent {} - Child {}", parent_idx, child_idx)),
                PrefabMarker,
                Transform::from_xyz(0.0, child_idx as f32 * 0.1, 0.0),
            )).id();
            commands.entity(parent).add_child(child);
        }
    }

    // 5 parents WITHOUT PrefabMarker
    for parent_idx in 0..5 {
        let parent = commands.spawn((
            Name::new(format!("Parent WITHOUT Prefab #{} (100 children)", parent_idx)),
            Transform::from_xyz(parent_idx as f32 * 3.0, 5.0, 0.0),
        )).id();

        for child_idx in 0..100 {
            let child = commands.spawn((
                Name::new(format!("Non-Prefab Parent {} - Child {}", parent_idx, child_idx)),
                Transform::from_xyz(0.0, child_idx as f32 * 0.1, 0.0),
            )).id();
            commands.entity(parent).add_child(child);
        }
    }

    // Add a few standalone root entities for good measure
    for i in 0..10 {
        commands.spawn((
            Name::new(format!("Standalone Root Entity {}", i)),
            PrefabMarker,
            Transform::from_xyz(i as f32 * 2.0, 10.0, 0.0),
        ));
    }
}