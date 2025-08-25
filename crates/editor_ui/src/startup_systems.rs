use bevy::prelude::*;
use bevy_egui::*;


fn install_image_loaders(mut ctx: EguiContexts) {
    if let Ok(first_ctx) = ctx.ctx_mut() {
        egui_extras::install_image_loaders(first_ctx);
    }
}

pub struct StartupSystems;

impl Plugin for StartupSystems {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, install_image_loaders);
    }
}
