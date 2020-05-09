use crate::assets::material::Material;
use crate::assets::{AssetManager, Handle};
use crate::resources::Resources;
use imgui::{im_str, ColorEdit, Selectable};

pub struct MaterialEditor {
    material: Option<Material>,
    selected_material: Option<String>,
}

impl Default for MaterialEditor {
    fn default() -> Self {
        Self {
            material: None,
            selected_material: None,
        }
    }
}
impl MaterialEditor {
    pub fn run_ui(&mut self, ui: &imgui::Ui, resources: &mut Resources) {
        let mut material_manager = resources.fetch_mut::<AssetManager<Material>>().unwrap();
        let all_materials: Vec<String> = material_manager.keys().map(|k| k.0.clone()).collect();

        if ui.small_button(im_str!("Select material..")) {
            ui.open_popup(im_str!("material_editor_select_material"));
        }

        // Material editor.`
        // -------------------------------------------
        if let (Some(material), Some(name)) =
            (self.material.as_mut(), self.selected_material.as_ref())
        {
            let mut edited = false;
            // color :)
            if ColorEdit::new(im_str!("Base color"), &mut material.base_color).build(ui) {
                edited = true;
            }

            if ui
                .input_float2(
                    im_str!("Metallic/roughness"),
                    &mut material.metallic_roughness_values,
                )
                .build()
            {
                edited = true;
            }

            if ColorEdit::new(im_str!("Emissive color"), &mut material.emissive_factor).build(ui) {
                edited = true;
            }

            if edited {
                if let Some(asset) = material_manager.get(&Handle(name.clone())) {
                    asset.execute_mut(|m| {
                        m.emissive_factor = material.emissive_factor;
                        m.base_color = material.base_color;
                        m.metallic_roughness_values = material.metallic_roughness_values;
                    })
                }
            }
        }

        // Popup to select what material to edit
        // -----------------------------------------
        ui.popup(im_str!("material_editor_select_material"), || {
            for name in all_materials.iter() {
                if Selectable::new(&im_str!("{}", name)).build(ui) {
                    if let Some(asset) = material_manager.get(&Handle(name.clone())) {
                        asset.execute(|m| {
                            self.selected_material = Some(name.clone());
                            self.material = Some(m.clone_strip());
                        })
                    }
                }
            }
        });
    }
}
