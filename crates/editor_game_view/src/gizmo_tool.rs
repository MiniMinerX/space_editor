use bevy::log::tracing_subscriber::filter::targets;
use bevy::{prelude::*};
use bevy_egui::egui::{self, Key};
use bevy_panorbit_camera::PanOrbitCamera;
use space_editor_core::prelude::*;
use space_editor_ui::{icons::*, EditorGizmo};
use space_editor_ui::prelude::CloneEvent;
use space_editor_ui::{colors::*, sizing::Sizing};
use space_shared::*;
use transform_gizmo_bevy::{EnumSet, Gizmo, GizmoCamera, GizmoMode, GizmoTarget, TransformGizmoPlugin};
use transform_gizmo_egui::GizmoExt;
use crate::*;
use crate::game_view_tool::*;
use space_editor_ui::prelude::update_pan_orbit;
use transform_gizmo_bevy::{GizmoOptions, GizmoOrientation};

pub struct GizmoToolPlugin;

impl Plugin for GizmoToolPlugin {
    #[cfg(not(tarpaulin_include))]
    fn build(&self, app: &mut App) {

        app.add_plugins(TransformGizmoPlugin);

        let gizmo_settings = GizmoOptions {
            gizmo_orientation: GizmoOrientation::Global,
            //pivot_point: TransformPivotPoint::IndividualOrigins,
            ..Default::default()
        };

        app.insert_resource(gizmo_settings);
        
        app.add_systems(Startup, add_gizmo_tool);
        app.add_observer(selected_trigger); 
        app.add_observer(selected_trigger_remove);

        app.add_systems(
            OnEnter(EditorState::Game),
            remove_gizmo_components,
        );

        app.add_systems(
            Update,
            disable_pan_orbit_on_gizmo
                .after(update_pan_orbit),
        );

        /* 
        app.editor_hotkey(GizmoHotkey::Translate, vec![KeyCode::KeyG]);
        app.editor_hotkey(GizmoHotkey::Rotate, vec![KeyCode::KeyR]);
        app.editor_hotkey(GizmoHotkey::Scale, vec![KeyCode::KeyS]);
        app.editor_hotkey(GizmoHotkey::Delete, vec![KeyCode::KeyX]);
        app.editor_hotkey(GizmoHotkey::Multiple, vec![KeyCode::ShiftLeft]);
        app.editor_hotkey(GizmoHotkey::Clone, vec![KeyCode::AltLeft]);
        */

    }
}

fn add_gizmo_tool(mut game_view_tab: ResMut<GameViewTab>) {
    game_view_tab.tools.push(Box::new(GizmoTool::default()));
    // Set the new tool as active
    game_view_tab.active_tool = Some(game_view_tab.tools.len() - 1);
}

fn remove_gizmo_components(
    mut commands: Commands,
    targets: Query<Entity, With<GizmoTarget>>,
) {
    for entity in targets.iter() {
        commands.entity(entity).remove::<GizmoTarget>();
    }
}

pub fn selected_trigger(
    trigger: On<Add, Selected>,
    mut commands: Commands,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
) {
    // Delete the previous gizmo target if it exists
    for (gizmo_target, gm_data) in gizmo_targets.iter() {
        commands.entity(gizmo_target).remove::<GizmoTarget>();
    }

    commands.entity(trigger.entity()).insert((
        GizmoTarget::default(),
    ));
}

 
pub fn selected_trigger_remove(
    trigger: On<Remove, Selected>,
    mut commands: Commands,
    selected_targets: Query<&Selected>,
) {
    // If a new selected target is made then we don't remove the gizmo target
    // In case we picked the same target again
    // But if no selected targets are left, we remove the gizmo target
    if selected_targets.is_empty() {
        commands.entity(trigger.entity()).remove::<GizmoTarget>();
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GizmoHotkey {
    Translate,
    Rotate,
    Scale,
    Delete,
    Multiple,
    Clone,
}

impl Hotkey for GizmoHotkey {
    fn name<'a>(&self) -> String {
        match self {
            Self::Translate => "Translate entity".to_string(),
            Self::Rotate => "Rotate entity".to_string(),
            Self::Scale => "Scale entity".to_string(),
            Self::Delete => "Delete entity".to_string(),
            Self::Multiple => "Change multiple entities".to_string(),
            Self::Clone => "Clone entity".to_string(),
        }
    }
}

 
pub struct GizmoTool {
    pub gizmo_mode: EnumSet<GizmoMode>,
}

impl Default for GizmoTool {
    fn default() -> Self {
        Self {
            gizmo_mode: GizmoMode::all_translate(),
        }
    }
}

const MODE_OPTIONS: [(EnumSet<GizmoMode>, &str, fn(&Sizing) -> egui::Button); 4] = [
    (GizmoMode::all_translate(), "Translate", |sizing| translate_icon(sizing.gizmos.to_size(), "T")),
    (GizmoMode::all_rotate(), "Rotate", |sizing| rotation_icon(sizing.gizmos.to_size(), "R")),
    (GizmoMode::all_scale(), "Scale", |sizing| scale_icon(sizing.gizmos.to_size(), "S")),
    (EnumSet::all(), "Universal", |sizing| universal_icon(sizing.gizmos.to_size(), "U")),
];



impl GameViewTool for GizmoTool {
    fn name(&self) -> &str {
        "Gizmo"
    }

    fn ui(
        &mut self, 
        ui: &mut egui::Ui, 
        commands: &mut Commands, 
        sizing: Res<Sizing>,
        mut gizmo_options: ResMut<GizmoOptions>,
    ) {

        ui.spacing();
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            let stl = ui.style_mut();
            stl.spacing.button_padding = egui::Vec2::new(4., 2.);
            stl.spacing.item_spacing = egui::Vec2::new(1., 0.);
            
            // Get current gizmo options
            
            for (mode, hint, button_fn) in MODE_OPTIONS {
                let is_current_mode = gizmo_options.gizmo_modes == mode;
                
                let button = button_fn(&sizing);
                let response = if is_current_mode {
                    ui.add(button.fill(SELECTED_ITEM_COLOR))
                } else {
                    ui.add(button)
                };
                
                if response.clicked() && !is_current_mode {
                    // Update the gizmo options resource
                    gizmo_options.gizmo_modes = mode;
                    self.gizmo_mode = mode;
                }
                
                response.on_hover_text(hint);
            }
        });

    }
}
    

  
