use crate::ecs::{Name, Transform};
use imgui::{im_str, Selectable};

mod components;
use crate::editor::components::{
    AmbientLightEditor, DirectionalLightEditor, NameEditor, RigidBodyEditor, TransformEditor,
};
use crate::physics::{BodyToEntity, PhysicWorld, RigidBody};
use crate::render::lighting::{AmbientLight, DirectionalLight};
use crate::resources::Resources;

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
        }
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
            .build(ui, || {
                for (e, _) in world.iter() {
                    let selected = self
                        .selected_entity
                        .map(|current| current == e)
                        .unwrap_or(false);

                    let entity_name = if let Ok(name) = world.get::<Name>(e) {
                        im_str!("{}", name.0)
                    } else {
                        im_str!("Entity {}", e.to_bits())
                    };
                    if Selectable::new(&entity_name).selected(selected).build(ui) {
                        self.selected_entity = Some(e);
                    }
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
                })
        }
    }
}
