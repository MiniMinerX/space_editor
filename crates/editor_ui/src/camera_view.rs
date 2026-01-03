use bevy::{
    camera::{RenderTarget, Viewport}, core_pipeline::tonemapping::DebandDither, prelude::*, render::{
        camera::{CameraRenderGraph, TemporalJitter},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    }, window::PrimaryWindow
};
use bevy_egui::{
    EguiContexts, EguiTextureHandle, egui::{self, RichText}
};

use space_prefab::component::PlaymodeCamera;
use space_shared::{toast::ToastMessage, *};

use crate::{
    DisableCameraSkip, RenderLayers, editor_tab_name::EditorTabName, prelude::GameModeSettings, ui_picking::NonUIAreas
};

use space_editor_tabs::prelude::*;

use crate::colors::*;

pub struct CameraViewTabPlugin;

impl Plugin for CameraViewTabPlugin {
    #[cfg(not(tarpaulin_include))]
    fn build(&self, app: &mut App) {
        use bevy_egui::EguiPrimaryContextPass;
        app.add_systems(
            EguiPrimaryContextPass,
            set_camera_view_non_ui_area
                .before(set_camera_viewport)
                .in_set(EditorSet::Editor),
        );
        app.editor_tab_by_trait(CameraViewTab::default());
        app.add_systems(EguiPrimaryContextPass, set_camera_viewport.in_set(EditorSet::Editor));
        app.add_systems(OnEnter(EditorState::Game), clean_camera_view_tab);
    }
}

#[derive(Resource, Default)]
pub struct CameraViewTab {
    /// egui rect in window space
    pub viewport_rect: Option<egui::Rect>,

    /// Which scene camera we are previewing
    pub camera_entity: Option<Entity>,

    /// The actual world camera rendering into the viewport
    pub preview_camera: Option<Entity>,
}


fn create_camera_image(width: u32, height: u32) -> Image {
    let size = Extent3d {
        width,
        height,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    image
}

impl EditorTab for CameraViewTab {
    fn ui(&mut self, ui: &mut bevy_egui::egui::Ui, commands: &mut Commands, world: &mut World) {
        ui.horizontal(|ui| {
            if self.preview_camera.is_none() {
                if world
                    .get_resource::<GameModeSettings>()
                    .map_or(false, |mode| mode.is_3d())
                {
                    self.preview_camera = Some(
                        commands
                            .spawn((
                                Camera3d::default(),
                                Camera {
                                    is_active: true,
                                    order: 99,
                                    clear_color: ClearColorConfig::None,
                                    ..default()
                                },
                                RenderLayers::layer(0),
                                TemporalJitter::default(),
                                Name::new("Camera for Camera view tab"),
                                DisableCameraSkip,
                                EditorCameraViewTabCamera,
                            ))
                            .id(),
                    );
                } else if world
                    .get_resource::<GameModeSettings>()
                    .map_or(false, |mode| mode.is_2d())
                {
                    self.preview_camera = Some(
                        commands
                            .spawn((
                                Camera2d::default(),
                                Camera {
                                    is_active: false,
                                    order: 99,
                                    clear_color: ClearColorConfig::Default,
                                    ..default()
                                },
                                RenderLayers::layer(0),
                                Name::new("Camera for Camera view tab"),
                                DisableCameraSkip,
                                EditorCameraViewTabCamera,
                            ))
                            .id(),
                    );
                }
            }

            let mut camera_query = world.query_filtered::<Entity, (
                With<Camera>,
                With<PlaymodeCamera>,
                //Without<EditorCameraMarker>,
            )>();

            if camera_query.iter(world).count() == 1 {
                let selected_entity = camera_query.iter(world).next();
                self.camera_entity = selected_entity;

                if let Some(entity) = selected_entity {
                    ui.label(format!("Camera: {:?}", entity));
                } else {
                    ui.label(RichText::new("No selected Camera").color(ERROR_COLOR));
                }
            } else if camera_query.iter(world).count() > 0 {
                egui::ComboBox::from_label("Camera")
                    .selected_text(format!("{:?}", self.camera_entity))
                    .show_ui(ui, |ui| {
                        for entity in camera_query.iter(world) {
                            ui.selectable_value(
                                &mut self.camera_entity,
                                Some(entity),
                                format!("{:?}", entity),
                            );
                        }
                    });
                ui.spacing();
                ui.separator();
            } else {
                ui.label(egui::RichText::new("No available Cameras").color(ERROR_COLOR));

                ui.spacing();
                ui.separator();
                ui.spacing();
                if world
                    .get_resource::<GameModeSettings>()
                    .map_or(false, |mode| mode.is_3d())
                {
                    ui.spacing();
                    if ui.button("Add 3D Playmode Camera").clicked() {
                        commands.spawn((
                            Camera3d::default(),
                            Camera::default(),
                            DebandDither::Enabled,
                            Projection::Perspective(PerspectiveProjection::default()),
                            Name::new("Camera3d".to_string()),
                            Transform::default(),
                            Visibility::default(),
                            PlaymodeCamera::default(),
                            PrefabMarker,
                            CameraRenderGraph::new(bevy::core_pipeline::core_3d::graph::Core3d),
                        ));
                    }
                } else if ui.button("Add 2D Playmode Camera").clicked() {
                    commands.spawn((
                        Camera2d {},
                        Name::new("Camera2d".to_string()),
                        Transform::default(),
                        Visibility::default(),
                        PlaymodeCamera::default(),
                        CameraRenderGraph::new(bevy::core_pipeline::core_2d::graph::Core2d),
                        PrefabMarker,
                    ));
                }
            }

            // Moves camera below the selection
            let pos = ui.next_widget_position();
            let mut clipped = ui.clip_rect();
            self.viewport_rect = Some(clipped);

            let mut need_recreate_texture = false;
        });
    }

    fn tab_name(&self) -> space_editor_tabs::tab_name::TabNameHolder {
        EditorTabName::CameraView.into()
    }
}

fn clean_camera_view_tab(
    mut ui_state: ResMut<CameraViewTab>,
    mut cameras: Query<(&mut Camera, &mut GlobalTransform) /*, Without<EditorCameraMarker> */ >,
) {
    let Some(real_cam_entity) = ui_state.preview_camera else {
        return;
    };

    let Ok((mut real_cam, _real_cam_transform)) = cameras.get_mut(real_cam_entity) else {
        return;
    };

    real_cam.is_active = false;
    real_cam.viewport = None;

    ui_state.camera_entity = None;
    ui_state.preview_camera = None;
    ui_state.viewport_rect = None;

    info!("Clean camera view tab successful");
}

#[derive(Default)]
struct LastCamTabRect(Option<egui::Rect>);

fn set_camera_viewport(
    mut local: Local<LastCamTabRect>,
    ui_state: Res<CameraViewTab>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera>,
) {
    let Ok(window) = primary_window.single() else { return };

    let Some(viewport_rect) = ui_state.viewport_rect else {
        local.0 = None;
        return;
    };

    // Avoid redundant writes
    if local.0 == Some(viewport_rect) {
        return;
    }
    local.0 = Some(viewport_rect);

    let Some(cam_entity) = ui_state.preview_camera else { return };
    let Ok(mut cam) = cameras.get_mut(cam_entity) else { return };

    let scale = window.scale_factor();
    let mut pos = viewport_rect.left_top().to_vec2() * scale;
    let mut size = viewport_rect.size() * scale;

    // Clamp to window
    pos.x = pos.x.max(0.0);
    pos.y = pos.y.max(0.0);

    let max_w = window.width() * scale - pos.x - 1.0;
    let max_h = window.height() * scale - pos.y - 1.0;

    size.x = size.x.min(max_w).max(1.0);
    size.y = size.y.min(max_h).max(1.0);

    cam.is_active = true;
    cam.viewport = Some(Viewport {
        physical_position: UVec2::new(pos.x as u32, pos.y as u32),
        physical_size: UVec2::new(size.x as u32, size.y as u32),
        depth: 0.0..1.0,
    });
}

fn set_camera_view_non_ui_area(
    mut non_ui_areas: ResMut<NonUIAreas>,
    cam_view: Res<CameraViewTab>,
) {
    if let Some(rect) = cam_view.viewport_rect {
        non_ui_areas.areas.push(rect);
    }
}
