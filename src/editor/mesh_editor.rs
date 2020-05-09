//! Mesh editor is a window that allows changing the materials of a mesh's primitive.

use crate::assets::material::Material;
use crate::assets::{Asset, AssetManager, Handle};
use crate::render::mesh::mesh::Mesh;
use crate::resources::Resources;
use imgui::{im_str, Selectable};

pub struct MeshEditor {
    selected_name: Option<String>,
    materials: Option<Vec<String>>,
}

impl Default for MeshEditor {
    fn default() -> Self {
        Self {
            selected_name: None,
            materials: None,
        }
    }
}

impl MeshEditor {
    pub fn run_ui(&mut self, ui: &imgui::Ui, resources: &mut Resources) {
        let mut mesh_manager = resources.fetch_mut::<AssetManager<Mesh>>().unwrap();
        let material_manager = resources.fetch::<AssetManager<Material>>().unwrap();
        let all_materials: Vec<String> = material_manager.keys().map(|k| k.0.clone()).collect();
        let assets: Vec<String> = mesh_manager.keys().map(|k| k.0.clone()).collect();

        if ui.small_button(im_str!("Select..")) {
            ui.open_popup(im_str!("select"));
        }

        let mut should_update_materials = false;
        if let (Some(materials), Some(asset_name)) =
            (self.materials.as_ref(), self.selected_name.as_ref())
        {
            for (i, mut p) in materials.iter().enumerate() {
                ui.text(&im_str!("Material: {:?}", p));

                if ui.small_button(&im_str!("Select material {}...", i)) {
                    ui.open_popup(&im_str!("select_material_{}_{}", asset_name, i));
                }

                ui.same_line(0.0);
                ui.text(&im_str!("{}", p));

                ui.popup(&im_str!("select_material_{}_{}", asset_name, i), || {
                    for m in all_materials.iter() {
                        if Selectable::new(&im_str!("{}", m))
                            .selected(*m == *p)
                            .build(ui)
                        {
                            if let Some(mut asset) =
                                mesh_manager.get_mut(&Handle(asset_name.clone()))
                            {
                                asset.execute_mut(|mut mesh| {
                                    mesh.primitives[i].material = Some(m.clone());
                                    should_update_materials = true;
                                })
                            }
                        }
                    }
                });
            }
        }

        if should_update_materials {
            if let Some(asset_name) = self.selected_name.as_ref() {
                let mut new_materials = vec![];
                if let Some(mut asset) = mesh_manager.get_mut(&Handle(asset_name.clone())) {
                    asset.execute_mut(|mut mesh| {
                        for p in mesh.primitives.iter() {
                            new_materials.push(
                                p.material
                                    .as_ref()
                                    .map(|m| m.clone())
                                    .unwrap_or("default_material".to_owned()),
                            );
                        }
                    })
                }

                self.materials = Some(new_materials);
            }
        }

        ui.popup(im_str!("select"), || {
            for name in assets.iter() {
                if Selectable::new(&im_str!("{}", name)).build(ui) {
                    let mut materials = vec![];
                    if let Some(asset) = mesh_manager.get(&Handle(name.clone())) {
                        asset.execute(|m| {
                            for p in m.primitives.iter() {
                                materials.push(
                                    p.material
                                        .as_ref()
                                        .map(|m| m.clone())
                                        .unwrap_or("default_material".to_owned()),
                                );
                            }
                        })
                    }
                    self.materials = Some(materials);
                    self.selected_name = Some(name.clone());
                }
            }
        });
    }
}
