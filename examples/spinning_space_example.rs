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

        .add_plugins(TransformGizmoPlugin)

        .add_systems(
            Update,
            disable_pan_orbit_on_gizmo
                .after(update_pan_orbit)
                .in_set(EditorSet::Editor),
        )

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


fn disable_pan_orbit_on_gizmo(
    mut pan_orbit_cams: Query<&mut PanOrbitCamera, With<GizmoCamera>>,
    gizmo_targets: Query<&GizmoTarget>,
) {
    for mut cam in pan_orbit_cams.iter_mut() {
        for gizmo_target in gizmo_targets.iter() {
            if gizmo_target.is_active() {
                cam.enabled = false;
                //debug!("Disabling PanOrbitCamera for GizmoTarget: {:?}", gizmo_target);
                return;
            }
        }
    }
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
            transform.rotate(Quat::from_axis_angle(
                marker.axis,
                marker.speed * time.delta_secs(),
            ));
        }
    }
}