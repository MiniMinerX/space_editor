use bevy::prelude::*;
use bevy::scene::{SceneInstanceReady, SceneSpawner};
use space_shared::PrefabMarker;

use crate::prelude::EditorRegistryExt;

use super::save::ChildrenPrefab;

/// Bundle for spawn prefabs
/// Example
///
/// commands.spawn(PrefabBundle::new("path/to/prefab"));
///
#[derive(Default, Bundle)]
pub struct PrefabBundle {
    pub loader: PrefabLoader,
    pub transform: Transform,
    pub global_transform: GlobalTransform,

    pub visibility: Visibility,
    pub computed_visibility: ViewVisibility,
    pub inherited_visibility: InheritedVisibility,
}

impl PrefabBundle {
    /// Create new prefab bundle from path to prefab file
    pub fn new(path: &str) -> Self {
        Self {
            loader: PrefabLoader {
                path: path.to_string(),
            },
            ..default()
        }
    }
}

/// Plugin for loading prefabs
pub struct LoadPlugin;

/// Marks all child of prefab to correct delete them when prefab is deleted
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component)]
pub struct PrefabAutoChild;

impl Plugin for LoadPlugin {
    #[cfg(not(tarpaulin_include))]
    fn build(&self, app: &mut App) {
        app.editor_registry::<PrefabLoader>();
        app.register_type::<PrefabAutoChild>();
        app.editor_registry::<PrefabAutoChild>();

        app.add_systems(
            Update,
            (
                conflict_resolve,
                load_prefab,
                ApplyDeferred,
                auto_children,
            )
                .chain(),
        );
    }
}

/// This component is mark that prefab should be loaded
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component)]
pub struct PrefabLoader {
    pub path: String,
}

/// System responsible for loading prefabs
fn load_prefab(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &PrefabLoader,
            Option<&Children>,
            Option<&Transform>,
            Option<&Visibility>,
        ),
        Changed<PrefabLoader>,
    >,
    auto_children: Query<Entity, With<PrefabAutoChild>>,
    assets: ResMut<AssetServer>,
) {
    for (e, l, children, tr, vis) in query.iter() {
        if tr.is_none() {
            commands
                .entity(e)
                .insert((Transform::default(), GlobalTransform::default()));
        }
        if vis.is_none() {
            commands.entity(e).insert(Visibility::default());
        }

        //remove old scene
        if let Some(children) = children {
            for child in children {
                if auto_children.contains(*child) {
                    commands.entity(*child).despawn();
                }
            }
            commands.entity(e).remove::<Children>();
        }
  
        let scene: Handle<DynamicScene> = assets.load(&l.path);

        let id = commands
            .spawn((DynamicSceneRoot(scene), PrefabAutoChild))
            .observe(prefab_scene_instance_ready)
            .id();

        commands.entity(e).add_children(&[id]);
        
        
    }
}

/// Observer for PrefabLoader: when scene instance is ready, add PrefabAutoChild to all instance entities.
fn prefab_scene_instance_ready(
    scene_ready: On<SceneInstanceReady>,
    scene_spawner: Res<SceneSpawner>,
    mut commands: Commands,
) {
    if scene_ready.entity == Entity::PLACEHOLDER || !scene_spawner.instance_is_ready(scene_ready.instance_id) {
        return;
    }

    for entity in scene_spawner.iter_instance_entities(scene_ready.instance_id) {
        commands.entity(entity).insert(PrefabAutoChild);
    }
}

fn conflict_resolve(
    mut commands: Commands,
    query: Query<Entity, (With<PrefabAutoChild>, With<PrefabMarker>)>,
) {
    for e in query.iter() {
        commands.entity(e).remove::<PrefabMarker>();
    }
}

  
fn auto_children(
    mut commands: Commands,
    query: Query<(Entity, &ChildrenPrefab)>,
    existing_entity: Query<Entity>,
) {
    for (e, children) in query.iter() {
        let mut cmds = commands.entity(e);
        for child in children.entities.iter() {
            println!("Adding child: {:?}", child);
            if existing_entity.contains(*child) {
                cmds.add_child(*child);
            } else {
                println!("nonexistent entity");
            }
        }
        cmds.remove::<ChildrenPrefab>();
    }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_prefab_bundler() {
        let bundler = PrefabBundle::new("path");

        assert_eq!(bundler.loader.path, "path");
    }

    #[test]
    fn conflict_resolver_only_one_prefab_component_allowed() {
        let mut app = App::new();

        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((PrefabAutoChild, PrefabMarker));
            commands.spawn(PrefabAutoChild);
        })
        .add_systems(Update, conflict_resolve);

        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<Entity, (With<PrefabAutoChild>, With<PrefabMarker>)>();
        assert_eq!(query.iter(&app.world()).count(), 0);

        let mut query = app
            .world_mut()
            .query_filtered::<Entity, With<PrefabAutoChild>>();
        assert_eq!(query.iter(&app.world()).count(), 2);
    }
}
