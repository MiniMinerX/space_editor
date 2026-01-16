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

        .run();
}

