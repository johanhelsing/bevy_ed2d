use bevy::{
    asset::{ReflectAsset, UntypedAssetId},
    color::palettes,
    prelude::*,
    reflect::TypeRegistry,
    render::{camera::Viewport, primitives::Aabb},
    window::PrimaryWindow,
};
use bevy_inspector_egui::{
    bevy_egui::{self, EguiContext, EguiPlugin, EguiSet},
    bevy_inspector::{
        self,
        hierarchy::{hierarchy_ui, SelectedEntities},
        ui_for_entities_shared_components, ui_for_entity_with_children,
    },
    DefaultInspectorConfigPlugin,
};
use bevy_mod_picking::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use egui_dock::{egui, DockArea, DockState, NodeIndex};
use std::any::TypeId;

pub struct Ed2dPlugin {
    pub auto_add_pickables: bool,
}

impl Default for Ed2dPlugin {
    fn default() -> Self {
        Self {
            auto_add_pickables: true,
        }
    }
}

impl Plugin for Ed2dPlugin {
    fn build(&self, app: &mut App) {
        // if !app.is_plugin_added::<DefaultPickingPlugins>() {
        app.add_plugins(DefaultPickingPlugins);
        // }
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        if !app.is_plugin_added::<PanCamPlugin>() {
            app.add_plugins(PanCamPlugin);
        }

        app.add_systems(Startup, setup)
            .add_systems(Update, (select_clicked, toggle_active))
            .add_systems(Update, draw_aabb_gizmos.run_if(is_ui_active))
            .add_systems(
                PostUpdate,
                show_ui_system
                    .run_if(is_ui_active)
                    .before(EguiSet::ProcessOutput)
                    .before(bevy::transform::TransformSystem::TransformPropagate),
            )
            .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
            .insert_resource(DebugPickingMode::Normal)
            .init_resource::<UiState>();

        if self.auto_add_pickables {
            app.add_systems(Update, auto_add_pickables);
        }
    }
}

#[derive(Component)]
struct Ed2dCamera;

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera2dBundle::default(),
        Ed2dCamera,
        PanCam {
            grab_buttons: vec![MouseButton::Middle, MouseButton::Right],
            ..default()
        },
    ));
}

// make camera only render to view not obstructed by UI
fn set_camera_viewport(
    ui_state: Res<UiState>,
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    egui_settings: Res<bevy_egui::EguiSettings>,
    mut cameras: Query<&mut Camera, With<Ed2dCamera>>,
) {
    let mut cam = cameras.single_mut();

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    if !ui_state.active {
        cam.viewport = None;
        return;
    }

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let viewport_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor;
    let viewport_size = ui_state.viewport_rect.size() * scale_factor;

    let physical_position = UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    let physical_size = UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    // The desired viewport rectangle at its offset in "physical pixel space"
    let rect = physical_position + physical_size;

    let window_size = window.physical_size();
    // wgpu will panic if trying to set a viewport rect which has coordinates extending
    // past the size of the render target, i.e. the physical window in our case.
    // Typically this shouldn't happen- but during init and resizing etc. edge cases might occur.
    // Simply do nothing in those cases.
    if rect.x <= window_size.x && rect.y <= window_size.y {
        cam.viewport = Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        });
    }
}

fn show_ui_system(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut())
    });
}

#[derive(Eq, PartialEq)]
enum InspectorSelection {
    Entities,
    Resource(TypeId, String),
    Asset(TypeId, String, UntypedAssetId),
}

#[derive(Resource)]
struct UiState {
    active: bool,
    state: DockState<EguiWindow>,
    viewport_rect: egui::Rect,
    selected_entities: SelectedEntities,
    selection: InspectorSelection,
    // gizmo_mode: GizmoMode,
}

impl Default for UiState {
    fn default() -> Self {
        let mut state = DockState::new(vec![EguiWindow::GameView]);
        let tree = state.main_surface_mut();
        let [game, _inspector] =
            tree.split_right(NodeIndex::root(), 0.75, vec![EguiWindow::Inspector]);
        let [_game, hierarchy] = tree.split_right(game, 0.75, vec![EguiWindow::Hierarchy]);

        let [_hierarchy, _resources_and_assets] = tree.split_below(
            hierarchy,
            0.6,
            vec![EguiWindow::Resources, EguiWindow::Assets],
        );

        Self {
            active: true,
            state,
            selected_entities: SelectedEntities::default(),
            selection: InspectorSelection::Entities,
            viewport_rect: egui::Rect::NOTHING,
            // gizmo_mode: GizmoMode::Translate,
        }
    }
}

impl UiState {
    fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selected_entities: &mut self.selected_entities,
            selection: &mut self.selection,
            // gizmo_mode: self.gizmo_mode,
        };
        DockArea::new(&mut self.state)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

#[derive(Debug)]
enum EguiWindow {
    GameView,
    Hierarchy,
    Resources,
    Assets,
    Inspector,
}

struct TabViewer<'a> {
    world: &'a mut World,
    selected_entities: &'a mut SelectedEntities,
    selection: &'a mut InspectorSelection,
    viewport_rect: &'a mut egui::Rect,
    // gizmo_mode: GizmoMode,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = EguiWindow;

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, window: &mut Self::Tab) {
        let type_registry = self.world.resource::<AppTypeRegistry>().0.clone();
        let type_registry = type_registry.read();

        match window {
            EguiWindow::GameView => {
                *self.viewport_rect = ui.clip_rect();

                // draw_gizmo(ui, self.world, self.selected_entities, self.gizmo_mode);
            }
            EguiWindow::Hierarchy => {
                let selected = hierarchy_ui(self.world, ui, self.selected_entities);
                if selected {
                    *self.selection = InspectorSelection::Entities;
                }
            }
            EguiWindow::Resources => select_resource(ui, &type_registry, self.selection),
            EguiWindow::Assets => select_asset(ui, &type_registry, self.world, self.selection),
            EguiWindow::Inspector => match *self.selection {
                InspectorSelection::Entities => match self.selected_entities.as_slice() {
                    &[entity] => ui_for_entity_with_children(self.world, entity, ui),
                    entities => ui_for_entities_shared_components(self.world, entities, ui),
                },
                InspectorSelection::Resource(type_id, ref name) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_resource(
                        self.world,
                        type_id,
                        ui,
                        name,
                        &type_registry,
                    )
                }
                InspectorSelection::Asset(type_id, ref name, handle) => {
                    ui.label(name);
                    bevy_inspector::by_type_id::ui_for_asset(
                        self.world,
                        type_id,
                        handle,
                        ui,
                        &type_registry,
                    );
                }
            },
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        format!("{window:?}").into()
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, EguiWindow::GameView)
    }
}

fn select_resource(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
) {
    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| {
            (
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
            )
        })
        .collect();
    resources.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));

    for (resource_name, type_id) in resources {
        let selected = match *selection {
            InspectorSelection::Resource(selected, _) => selected == type_id,
            _ => false,
        };

        if ui.selectable_label(selected, resource_name).clicked() {
            *selection = InspectorSelection::Resource(type_id, resource_name.to_string());
        }
    }
}

fn select_asset(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    world: &World,
    selection: &mut InspectorSelection,
) {
    let mut assets: Vec<_> = type_registry
        .iter()
        .filter_map(|registration| {
            let reflect_asset = registration.data::<ReflectAsset>()?;
            Some((
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
                reflect_asset,
            ))
        })
        .collect();
    assets.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));

    for (asset_name, asset_type_id, reflect_asset) in assets {
        let handles: Vec<_> = reflect_asset.ids(world).collect();

        ui.collapsing(format!("{asset_name} ({})", handles.len()), |ui| {
            for handle in handles {
                let selected = match *selection {
                    InspectorSelection::Asset(_, _, selected_id) => selected_id == handle,
                    _ => false,
                };

                if ui
                    .selectable_label(selected, format!("{:?}", handle))
                    .clicked()
                {
                    *selection =
                        InspectorSelection::Asset(asset_type_id, asset_name.to_string(), handle);
                }
            }
        });
    }
}

fn select_clicked(
    mut ui_state: ResMut<UiState>,
    mut clicks: EventReader<Pointer<Click>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for click in clicks.read() {
        println!("Clicked: {:?}", click);
        // select the clicked entity in the inspector
        let clicked_entity = click.target;

        let add = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
        ]);

        // select in the inspector
        ui_state.selection = InspectorSelection::Entities;
        ui_state
            .selected_entities
            .select_maybe_add(clicked_entity, add);
    }
}

fn toggle_active(mut ui_state: ResMut<UiState>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        ui_state.active = !ui_state.active;
    }
}

fn is_ui_active(ui_state: Res<UiState>) -> bool {
    ui_state.active
}

fn auto_add_pickables(
    mut commands: Commands,
    query: Query<Entity, (Without<Pickable>, With<Sprite>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(PickableBundle::default());
    }
}

// fn handle_pick_events(
//     mut ui_state: ResMut<UiState>,
//     mut click_events: EventReader<PointerClick>,
//     mut egui: ResMut<EguiContext>,
//     egui_entity: Query<&EguiPointer>,
// ) {
//     let egui_context = egui.ctx_mut();

//     for click in click_events.iter() {
//         if egui_entity.get(click.target()).is_ok() {
//             continue;
//         };

//         let modifiers = egui_context.input().modifiers;
//         let add = modifiers.ctrl || modifiers.shift;

//         ui_state
//             .selected_entities
//             .select_maybe_add(click.target(), add);
//     }
// }

// fn set_gizmo_mode(input: Res<ButtonInput<KeyCode>>, mut ui_state: ResMut<UiState>) {
//     for (key, mode) in [
//         (KeyCode::KeyR, GizmoMode::Rotate),
//         (KeyCode::KeyT, GizmoMode::Translate),
//         (KeyCode::KeyS, GizmoMode::Scale),
//     ] {
//         if input.just_pressed(key) {
//             ui_state.gizmo_mode = mode;
//         }
//     }
// }

// #[allow(unused)]
// fn draw_gizmo(
//     ui: &mut egui::Ui,
//     world: &mut World,
//     selected_entities: &SelectedEntities,
//     gizmo_mode: GizmoMode,
// ) {
//     let (cam_transform, projection) = world
//         .query_filtered::<(&GlobalTransform, &Projection), With<MainCamera>>()
//         .single(world);
//     let view_matrix = Mat4::from(cam_transform.affine().inverse());
//     let projection_matrix = projection.get_clip_from_view();

//     if selected_entities.len() != 1 {
//         return;
//     }

//     /*for selected in selected_entities.iter() {
//         let Some(transform) = world.get::<Transform>(selected) else {
//             continue;
//         };
//         let model_matrix = transform.compute_matrix();

//         let mut gizmo = Gizmo::new(GizmoConfig {
//             view_matrix: view_matrix.into(),
//             projection_matrix: projection_matrix.into(),
//             orientation: GizmoOrientation::Local,
//             modes: EnumSet::from(gizmo_mode),
//             ..Default::default()
//         });
//         let Some([result]) = gizmo
//             .interact(ui, model_matrix.into())
//             .map(|(_, res)| res.as_slice())
//         else {
//             continue;
//         };

//         let mut transform = world.get_mut::<Transform>(selected).unwrap();
//         transform = Transform {
//             translation: Vec3::from(<[f64; 3]>::from(result.translation)),
//             rotation: Quat::from_array(<[f64; 4]>::from(result.rotation)),
//             scale: Vec3::from(<[f64; 3]>::from(result.scale)),
//         };
//     }*/
// }

fn draw_aabb_gizmos(mut gizmos: Gizmos, aabbs: Query<(&Aabb, &GlobalTransform, &PickSelection)>) {
    for (aabb, transform, pick_selection) in &aabbs {
        if !pick_selection.is_selected {
            continue;
        }

        let (scale, rotation, translation) = transform.to_scale_rotation_translation();
        let size = scale.xy() * aabb.half_extents.xy() * 2.0;
        let color = palettes::basic::LIME;
        gizmos.rect(translation, rotation, size, color)
    }
}
