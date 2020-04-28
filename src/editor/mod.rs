use crate::ecs::{Name, Transform};
use imgui::{im_str, Selectable};

mod components;
use components::{NameEditor, TransformEditor};
/// Keep the state of the game editor.
#[derive(Default)]
pub struct Editor {
    selected_entity: Option<hecs::Entity>,

    // Editors for the components.
    transform_editor: TransformEditor,
    name_editor: NameEditor,
}

impl Editor {
    pub fn show_components(&mut self, ui: &imgui::Ui, world: &hecs::World) {
        imgui::Window::new(im_str!("Entities"))
            .opened(&mut true)
            .position([0.0, 0.0], imgui::Condition::Always)
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
                .position([500.0, 0.0], imgui::Condition::FirstUseEver)
                .build(ui, || {
                    ui.text(im_str!("Selected entity is {:?}", entity));

                    if let Ok(mut t) = world.get_mut::<Transform>(entity) {
                        self.transform_editor.edit(ui, &mut t);
                    }

                    if let Ok(mut n) = world.get_mut::<Name>(entity) {
                        self.name_editor.edit(ui, &mut n);
                    }
                })
        }
    }
}
