use bevy::{
    asset::{AssetPath, LoadState},
    gltf::{Gltf, GltfMesh, GltfNode},
    platform::collections::HashMap,
    prelude::*,
};

use space_prefab::component::{AssetMaterial, AssetMesh, Mesh3dMaterialPrefab};
use space_shared::PrefabMarker;
use space_shared::toast::{ToastKind, ToastMessage};

use super::{BackgroundTask, BackgroundTaskStorage};

#[derive(Message)]
/// Event to handle GLTF path
pub struct EditorUnpackGltf {
    pub path: String,
    pub parent: Option<Entity>,
}

#[derive(Message, Clone)]
struct GltfLoaded {
    handle: Handle<Gltf>,
    parent: Option<Entity>,
}

pub struct UnpackGltfPlugin;

impl Plugin for UnpackGltfPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EditorUnpackGltf>();
        app.add_message::<GltfLoaded>();
        app.add_systems(PreUpdate, (unpack_gltf_event, queue_push, unpack_gltf));

        app.init_resource::<GltfSceneQueue>();

        app.register_type::<GltfHolder>();
    }
}

#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component)]
struct GltfHolder(Handle<Gltf>);

#[derive(Resource, Default)]  // Handle then parent
struct GltfSceneQueue(Vec<(Handle<Gltf>, Option<Entity>)>);

fn unpack_gltf_event(
    mut events: MessageReader<EditorUnpackGltf>,
    assets: Res<AssetServer>,
    mut queue: ResMut<GltfSceneQueue>,
    mut background_tasks: ResMut<BackgroundTaskStorage>,
) {
    for event in events.read() {
        let handle = assets.load(event.path.clone());
        background_tasks.tasks.push(BackgroundTask::AssetLoading(
            event.path.clone(),
            handle.clone().untyped(),
        ));
        queue.0.push((handle, event.parent));
    }
    events.clear();
}

// separated from unpack_gltf for reduce arguments count and ordered unpack
fn queue_push(
    mut queue: ResMut<GltfSceneQueue>,
    mut events: MessageWriter<GltfLoaded>,
    assets: Res<AssetServer>,
) {
    if let Some((handle, parent)) = queue.0.first().cloned() {
        if matches!(assets.get_load_state(&handle), Some(LoadState::Loaded)) {
            events.write(GltfLoaded { handle, parent });
            queue.0.remove(0);
        }
    }
}

struct UnpackContext<'a> {
    material_map: &'a HashMap<Handle<StandardMaterial>, usize>,
    mesh_map: &'a HashMap<Handle<GltfMesh>, usize>,
    gltf_meshs: &'a Assets<GltfMesh>,
    gltf_path: &'a AssetPath<'a>,
    gltf_nodes: &'a Assets<GltfNode>,
}

fn unpack_gltf(
    mut gltf_loaded: MessageReader<GltfLoaded>,
    gltf_assets: Res<Assets<Gltf>>,
    node_assets: Res<Assets<GltfNode>>,
    mesh_assets: Res<Assets<GltfMesh>>,
    mut toast: MessageWriter<ToastMessage>,
    mut commands: Commands,
) {
    let loaded: Vec<GltfLoaded> = gltf_loaded.read().cloned().collect();
    gltf_loaded.clear();

    for gltf_loaded in loaded {
        let handle = gltf_loaded.handle.clone();
        let Some(gltf_path) = handle.path().cloned() else {
            continue;
        };
        info!("Path: {:?}", &gltf_path);

        let Some(gltf) = gltf_assets.get(&handle) else {
            toast.write(ToastMessage::new(
                "Gltf asset not found or empty",
                ToastKind::Error,
            ));
            continue;
        };

        let mut mesh_map = HashMap::new();
        for (idx, h) in gltf.meshes.iter().enumerate() {
            mesh_map.insert(h.clone(), idx);
        }

        let mut material_map = HashMap::new();
        for (idx, h) in gltf.materials.iter().enumerate() {
            info!("Material: {:?}", h);
            material_map.insert(h.clone(), idx);
        }

        // Root detection from Gltf node graph (no Scene world usage)
        let mut has_parent: HashMap<Handle<GltfNode>, ()> = HashMap::default();
        for node_handle in &gltf.nodes {
            if let Some(node) = node_assets.get(node_handle) {
                for child in &node.children {
                    has_parent.insert(child.clone(), ());
                }
            }
        }
        let roots: Vec<Handle<GltfNode>> = gltf
            .nodes
            .iter()
            .filter(|h| !has_parent.contains_key(*h))
            .cloned()
            .collect();

        info!("Roots: {:?}", &roots);

        let ctx = UnpackContext {
            material_map: &material_map,
            mesh_map: &mesh_map,
            gltf_meshs: &*mesh_assets,
            gltf_path: &gltf_path,
            gltf_nodes: &*node_assets,
        };

        for root in &roots {
            let entity = spawn_node(&mut commands, root, gltf, &ctx);
            if let Some(parent) = gltf_loaded.parent {
                commands.entity(parent).add_child(entity);
            }
        }
    }
}

fn spawn_node(
    commands: &mut Commands,
    node_handle: &Handle<GltfNode>,
    _gltf: &Gltf,
    ctx: &UnpackContext<'_>,
) -> Entity {
    
    let gltf_nodes = ctx.gltf_nodes;

    let Some(node) = gltf_nodes.get(node_handle) else {
        error!("Failed to get GltfNode for handle: {:?}", node_handle);
        return commands.spawn_empty().id();
    };


    let id = commands
        .spawn((
            node.transform,
            Visibility::default(),
            PrefabMarker,
        ))
        .id();

    if let Some(handle) = &node.mesh {
        if let Some(mesh) = ctx.gltf_meshs.get(handle) {
            if mesh.primitives.len() == 1 {
                commands.entity(id).insert(AssetMesh {
                    path: format!(
                        "{}#Mesh{}/Primitive{}",
                        ctx.gltf_path.path().display(),
                        ctx.mesh_map.get(handle).unwrap(),
                        0
                    ),
                });

                if let Some(material_handle) = &mesh.primitives[0].material {
                    if let Some(idx) = ctx.material_map.get(material_handle) {
                        commands.entity(id).insert(AssetMaterial {
                            path: format!("{}#Material{}", ctx.gltf_path.path().display(), idx),
                        });
                    } else {
                        commands.entity(id).insert(Mesh3dMaterialPrefab::default());
                    }
                } else {
                    commands.entity(id).insert(Mesh3dMaterialPrefab::default());
                }
            } else {
                commands.entity(id).with_children(|parent| {
                    for idx in 0..mesh.primitives.len() {
                        let mut id = parent.spawn((
                            Transform::default(),
                            Visibility::default(),
                            AssetMesh {
                                path: format!(
                                    "{}#Mesh{}/Primitive{}",
                                    ctx.gltf_path.path().display(),
                                    ctx.mesh_map.get(handle).unwrap(),
                                    idx
                                ),
                            },
                            PrefabMarker,
                        ));

                        if let Some(material_handle) = &mesh.primitives[idx].material {
                            if let Some(idx) = ctx.material_map.get(material_handle) {
                                id.insert(AssetMaterial {
                                    path: format!(
                                        "{}#Material{}",
                                        ctx.gltf_path.path().display(),
                                        idx
                                    ),
                                });
                            } else {
                                id.insert(Mesh3dMaterialPrefab::default());
                            }
                        } else {
                            id.insert(Mesh3dMaterialPrefab::default());
                        }
                    }
                });
            }
        }
    }

    for child in &node.children {
        let child_id = spawn_node(commands, child, _gltf, ctx);
        commands.entity(id).add_child(child_id);
    }

    id
}
