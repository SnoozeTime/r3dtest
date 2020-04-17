use super::EditorComponent;
use crate::ecs::serialization::SerializedEntity;
use crate::editor::{ColorEditor, ColorMessage, FloatVec3Editor, FloatVec3Message};
use crate::physics::{BodyType, RigidBody, Shape};
use crate::render::billboard::Billboard;
use crate::render::lighting::{AmbientLight, DirectionalLight, Emissive, PointLight};
use iced::{text_input, Checkbox, Column, Command, Element, Radio, Row, Text, TextInput};

// RIGID BODY COMPONENT
// ================================================================================================

#[derive(Debug, Clone)]
pub enum RigidBodyMessage {
    AabbChanged(FloatVec3Message),
    RadioSelected(BodyType),
}

#[derive(Default)]
pub struct RigidBodyEditor {
    pub rigid_body: RigidBody,
    aabb_editor: Option<FloatVec3Editor>,
    // radio buttons to choose between STATIC, DYNAMIC and KINEMATIC
}

impl RigidBodyEditor {
    pub fn new(rigid_body: RigidBody) -> Self {
        let aabb_editor = if let Shape::AABB(halfextend) = rigid_body.shape {
            Some(FloatVec3Editor::new(halfextend))
        } else {
            None
        };
        Self {
            rigid_body,
            aabb_editor,
        }
    }
}

impl EditorComponent for RigidBodyEditor {
    type Message = RigidBodyMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            RigidBodyMessage::AabbChanged(msg) => {
                if let Some(ref mut aabb_editor) = self.aabb_editor {
                    if let Shape::AABB(ref mut halfextend) = self.rigid_body.shape {
                        aabb_editor.update(msg, halfextend);
                    }
                }
            }
            RigidBodyMessage::RadioSelected(body_type) => {
                self.rigid_body.ty = body_type;
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut column = Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("RigidBody").size(10));
        if let Some(aabb_editor) = self.aabb_editor.as_mut() {
            if let Shape::AABB(halfextend) = self.rigid_body.shape {
                column = column.push(
                    aabb_editor
                        .view(halfextend)
                        .map(RigidBodyMessage::AabbChanged),
                );
            }
        }

        let selected_ty = Some(self.rigid_body.ty);
        let row = Row::new()
            .spacing(1)
            .padding(1)
            .push(Radio::new(
                BodyType::Dynamic,
                "Dynamic",
                selected_ty,
                RigidBodyMessage::RadioSelected,
            ))
            .push(Radio::new(
                BodyType::Static,
                "Static",
                selected_ty,
                RigidBodyMessage::RadioSelected,
            ))
            .push(Radio::new(
                BodyType::Kinematic,
                "Kinematic",
                selected_ty,
                RigidBodyMessage::RadioSelected,
            ));

        column.push(row).into()
    }

    fn name(&self) -> String {
        "rigid_body".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.rigid_body = Some(self.rigid_body.clone());
    }
}

// POINT LIGHT Component
// ================================================================================================

#[derive(Debug, Clone)]
pub enum PointLightMessage {
    ColorChanged(ColorMessage),
}

#[derive(Default)]
pub struct PointLightEditor {
    pub point_light: PointLight,
    color_editor: ColorEditor,
}

impl PointLightEditor {
    pub fn new(point_light: PointLight) -> Self {
        Self {
            point_light,
            color_editor: ColorEditor::new(),
        }
    }
}

impl EditorComponent for PointLightEditor {
    type Message = PointLightMessage;

    fn update(&mut self, msg: Self::Message) -> Command<Self::Message> {
        match msg {
            PointLightMessage::ColorChanged(msg) => {
                self.color_editor.update(msg, &mut self.point_light.color)
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("PointLight").size(10))
            .push(
                self.color_editor
                    .view(self.point_light.color)
                    .map(move |msg| PointLightMessage::ColorChanged(msg)),
            )
            .into()
    }

    fn name(&self) -> String {
        "point_light".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.point_light = Some(self.point_light);
    }
}

// Emissive Component
// ================================================================================================

#[derive(Debug, Clone)]
pub enum EmissiveMessage {
    ColorChanged(ColorMessage),
}

#[derive(Default)]
pub struct EmissiveEditor {
    pub emissive: Emissive,
    color_editor: ColorEditor,
}

impl EmissiveEditor {
    pub fn new(emissive: Emissive) -> Self {
        Self {
            emissive,
            color_editor: ColorEditor::new(),
        }
    }
}

impl EditorComponent for EmissiveEditor {
    type Message = EmissiveMessage;

    fn update(&mut self, msg: Self::Message) -> Command<Self::Message> {
        match msg {
            EmissiveMessage::ColorChanged(msg) => {
                self.color_editor.update(msg, &mut self.emissive.color)
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("Emissive").size(10))
            .push(
                self.color_editor
                    .view(self.emissive.color)
                    .map(move |msg| EmissiveMessage::ColorChanged(msg)),
            )
            .into()
    }

    fn name(&self) -> String {
        "emissive".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.emissive = Some(self.emissive);
    }
}

// DirectionalLight Component
// ================================================================================================

#[derive(Debug, Clone)]
pub enum DirectionalLightMessage {
    ColorChanged(ColorMessage),
    IntensityChanged(String),
    DirectionalChanged(FloatVec3Message),
}

#[derive(Default)]
pub struct DirectionalLightEditor {
    pub directional_light: DirectionalLight,
    color_editor: ColorEditor,
    intensity_state: text_input::State,
    direction_editor: FloatVec3Editor,
}

impl DirectionalLightEditor {
    pub fn new(directional_light: DirectionalLight) -> Self {
        Self {
            directional_light,
            color_editor: ColorEditor::new(),
            intensity_state: text_input::State::default(),
            direction_editor: FloatVec3Editor::new(directional_light.direction),
        }
    }
}

impl EditorComponent for DirectionalLightEditor {
    type Message = DirectionalLightMessage;

    fn update(&mut self, msg: Self::Message) -> Command<Self::Message> {
        match msg {
            DirectionalLightMessage::ColorChanged(msg) => self
                .color_editor
                .update(msg, &mut self.directional_light.color),
            DirectionalLightMessage::IntensityChanged(new_intensity) => {
                if let Ok(new_intensity) = new_intensity.parse::<f32>() {
                    self.directional_light.intensity = new_intensity;
                }
            }
            DirectionalLightMessage::DirectionalChanged(msg) => self
                .direction_editor
                .update(msg, &mut self.directional_light.direction),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("DirectionalLight").size(10))
            .push(
                self.color_editor
                    .view(self.directional_light.color)
                    .map(move |msg| DirectionalLightMessage::ColorChanged(msg)),
            )
            .push(
                TextInput::new(
                    &mut self.intensity_state,
                    "intensity",
                    &format!("{}", self.directional_light.intensity),
                    DirectionalLightMessage::IntensityChanged,
                )
                .size(10),
            )
            .push(
                self.direction_editor
                    .view(self.directional_light.direction)
                    .map(DirectionalLightMessage::DirectionalChanged),
            )
            .into()
    }

    fn name(&self) -> String {
        "directional_light".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.directional_light = Some(self.directional_light);
    }
}

// Ambient Component
// ================================================================================================

#[derive(Debug, Clone)]
pub enum AmbientMessage {
    ColorChanged(ColorMessage),
    IntensityChanged(String),
}

#[derive(Default)]
pub struct AmbientEditor {
    pub ambient: AmbientLight,
    color_editor: ColorEditor,
    intensity_state: text_input::State,
}

impl AmbientEditor {
    pub fn new(ambient: AmbientLight) -> Self {
        Self {
            ambient,
            color_editor: ColorEditor::new(),
            intensity_state: text_input::State::default(),
        }
    }
}

impl EditorComponent for AmbientEditor {
    type Message = AmbientMessage;

    fn update(&mut self, msg: Self::Message) -> Command<Self::Message> {
        match msg {
            AmbientMessage::ColorChanged(msg) => {
                self.color_editor.update(msg, &mut self.ambient.color)
            }
            AmbientMessage::IntensityChanged(new_intensity) => {
                if let Ok(new_intensity) = new_intensity.parse::<f32>() {
                    self.ambient.intensity = new_intensity;
                }
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("Ambient").size(10))
            .push(
                self.color_editor
                    .view(self.ambient.color)
                    .map(move |msg| AmbientMessage::ColorChanged(msg)),
            )
            .push(
                TextInput::new(
                    &mut self.intensity_state,
                    "intensity",
                    &format!("{}", self.ambient.intensity),
                    AmbientMessage::IntensityChanged,
                )
                .size(10),
            )
            .into()
    }

    fn name(&self) -> String {
        "ambient".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.ambient_light = Some(self.ambient);
    }
}

// RENDER EDITOR
// =================================================================================================
/// A name is just a string input
#[derive(Default)]
pub struct BillboardEditor {
    billboard: Billboard,
    state: text_input::State,
    sprite_nb_state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum BillboardMessage {
    ToggleEnable(bool),
    MeshChanged(String),
    SpriteNbChanged(String),
}

impl BillboardEditor {
    pub fn new(billboard: Billboard) -> Self {
        Self {
            billboard,
            state: text_input::State::default(),
            sprite_nb_state: text_input::State::default(),
        }
    }
}

impl EditorComponent for BillboardEditor {
    type Message = BillboardMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            BillboardMessage::MeshChanged(new_mesh) => self.billboard.texture = new_mesh,
            BillboardMessage::ToggleEnable(new_enabled) => self.billboard.enabled = new_enabled,
            BillboardMessage::SpriteNbChanged(new_sprite_nb) => {
                if let Ok(sprite_nb) = new_sprite_nb.parse::<usize>() {
                    self.billboard.sprite_nb = sprite_nb;
                }
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("Billboard").size(10))
            .push(
                TextInput::new(
                    &mut self.state,
                    "",
                    &self.billboard.texture,
                    BillboardMessage::MeshChanged,
                )
                .padding(1)
                .size(10),
            )
            .push(
                Checkbox::new(
                    self.billboard.enabled,
                    "enabled?",
                    BillboardMessage::ToggleEnable,
                )
                .size(10),
            )
            .push(
                TextInput::new(
                    &mut self.sprite_nb_state,
                    "sprite number",
                    &format!("{}", self.billboard.sprite_nb),
                    BillboardMessage::SpriteNbChanged,
                )
                .size(10),
            )
            .into()
    }

    fn name(&self) -> String {
        String::from("billboard")
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.billboard = Some(self.billboard.clone());
    }
}
