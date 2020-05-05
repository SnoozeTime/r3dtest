use crate::ecs::{Name, Transform};
use imgui::{im_str, ImString, MenuItem, Selectable, StyleVar, TreeNode};

mod components;
use crate::editor::components::{
    AmbientLightEditor, DirectionalLightEditor, LocalTransformEditor, NameEditor, RenderEditor,
    RigidBodyEditor, TransformEditor,
};
use crate::physics::{BodyToEntity, PhysicWorld, RigidBody};
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::render::Render;
use crate::resources::Resources;
use crate::transform::{HasChildren, HasParent, LocalTransform};

/// Keep the state of the game editor.
pub struct Editor {
    selected_entity: Option<hecs::Entity>,

    // size of the screen
    w: u32,
    h: u32,

    // Editors for the components.
    transform_editor: TransformEditor,
    name_editor: NameEditor,
    rigidbody_editor: RigidBodyEditor,

    // Loading GLTF
    current_gltf_to_load: ImString,
    pub gltf_to_load: Option<String>,
}

impl Editor {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            selected_entity: None,
            transform_editor: TransformEditor::default(),
            name_editor: NameEditor::default(),
            rigidbody_editor: RigidBodyEditor::default(),
            current_gltf_to_load: ImString::with_capacity(128),
            gltf_to_load: None,
        }
    }

    fn build_tree(
        &mut self,
        world: &hecs::World,
        parent: hecs::Entity,
        children: Vec<hecs::Entity>,
        ui: &imgui::Ui,
    ) {
        let entity_name = if let Ok(name) = world.get::<Name>(parent) {
            im_str!("{}", name.0)
        } else {
            im_str!("Entity {}", parent.to_bits())
        };

        let selected = self
            .selected_entity
            .map(|current| current == parent)
            .unwrap_or(false);

        TreeNode::new(ui, &entity_name)
            .leaf(children.is_empty())
            .selected(selected)
            .opened(true, imgui::Condition::FirstUseEver)
            .open_on_double_click(true)
            .open_on_arrow(true)
            .build(|| {
                if ui.is_item_clicked(imgui::MouseButton::Left) {
                    self.selected_entity = Some(parent);
                }
                for c in children {
                    let gc = if let Ok(gc) = world.get::<HasChildren>(c) {
                        gc.children.clone()
                    } else {
                        vec![]
                    };

                    self.build_tree(world, c, gc, ui);
                }
            });
    }

    pub fn show_components(
        &mut self,
        ui: &imgui::Ui,
        world: &hecs::World,
        physics: &mut PhysicWorld,
        resources: &mut Resources,
    ) {
        imgui::Window::new(im_str!("Entities"))
            .opened(&mut true)
            .position([10.0, 10.0], imgui::Condition::FirstUseEver)
            .size([200.0, 500.0], imgui::Condition::FirstUseEver)
            .build(ui, || {
                if ui.button(im_str!("Import.."), [0.0, 0.0]) {
                    ui.open_popup(im_str!("Import?"));
                }
                self.show_load_gltf_popup(ui);

                let parent_nodes: Vec<(hecs::Entity, Vec<hecs::Entity>)> = world
                    .iter()
                    .filter(|(e, _)| {
                        let has_parent = world.get::<HasParent>(*e);
                        has_parent.is_err()
                    })
                    .map(|(e, _)| {
                        let children = if let Ok(cc) = world.get::<HasChildren>(e) {
                            cc.children.clone()
                        } else {
                            vec![]
                        };

                        (e, children)
                    })
                    .collect();

                for (parent, children) in parent_nodes {
                    self.build_tree(world, parent, children, ui);
                }
            });

        if let Some(entity) = self.selected_entity {
            imgui::Window::new(im_str!("Components"))
                .opened(&mut true)
                .position(
                    [self.w as f32 - 300.0, 10.0],
                    imgui::Condition::FirstUseEver,
                )
                .size([250.0, 400.0], imgui::Condition::FirstUseEver)
                .build(ui, || {
                    ui.text(im_str!("Selected entity is {:?}", entity));

                    if let Ok(mut t) = world.get_mut::<Transform>(entity) {
                        self.transform_editor.edit(ui, &mut t);
                    }

                    if let Ok(mut t) = world.get_mut::<LocalTransform>(entity) {
                        LocalTransformEditor::default().edit(ui, &mut t);
                    }

                    if let Ok(mut n) = world.get_mut::<Name>(entity) {
                        self.name_editor.edit(ui, &mut n);
                    }

                    if let Ok(mut rb) = world.get_mut::<RigidBody>(entity) {
                        if let Ok(t) = world.get::<Transform>(entity) {
                            if self.rigidbody_editor.edit(ui, &mut rb) {
                                // edited.
                                let mut body_to_entity =
                                    resources.fetch_mut::<BodyToEntity>().unwrap();

                                if let Some(h) = rb.handle {
                                    body_to_entity.remove(&h);
                                }
                                physics.update_rigidbody_component(&t, &mut rb);
                            }
                        }
                    }

                    if let Ok(mut ambient) = world.get_mut::<AmbientLight>(entity) {
                        AmbientLightEditor::default().edit(ui, &mut ambient);
                    }

                    if let Ok(mut light) = world.get_mut::<DirectionalLight>(entity) {
                        DirectionalLightEditor::default().edit(ui, &mut light);
                    }

                    if let Ok(mut render) = world.get_mut::<Render>(entity) {
                        RenderEditor::default().edit(ui, &mut render);
                    }
                })
        }
    }

    fn show_load_gltf_popup(&mut self, ui: &imgui::Ui) {
        ui.popup_modal(im_str!("Import?"))
            .always_auto_resize(true)
            .build(|| {
                ui.text("Choose path where ther gltf file is located:\n");
                ui.separator();
                let style = ui.push_style_var(StyleVar::FramePadding([0.0, 0.0]));
                ui.input_text(im_str!("File to load:"), &mut self.current_gltf_to_load)
                    .build();
                if ui.button(im_str!("OK"), [120.0, 0.0]) {
                    self.gltf_to_load = Some(self.current_gltf_to_load.to_string());
                    self.current_gltf_to_load.clear();
                    ui.close_current_popup();
                }
                ui.same_line(0.0);
                if ui.button(im_str!("Cancel"), [120.0, 0.0]) {
                    self.current_gltf_to_load.clear();
                    ui.close_current_popup();
                }
                style.pop(ui);
            });
    }
}
