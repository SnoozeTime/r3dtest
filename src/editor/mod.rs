use crate::ecs::{serialization::SerializedEntity, Name, Transform};
use crate::physics::{RigidBody, Shape};
use crate::render::{
    lighting::{Emissive, PointLight},
    Render,
};
use glam::Quat;
use iced::{
    button, text_input, Align, Application, Button, Checkbox, Column, Command, Element, Row, Text,
    TextInput,
};
use nalgebra::UnitQuaternion;

mod components;
use crate::colors::RgbColor;
use components::*;

pub trait EditorComponent: Default {
    type Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message>;
    fn view(&mut self) -> Element<Self::Message>;
    fn name(&self) -> String;
    fn apply(&self, entity: &mut SerializedEntity);
}

#[derive(Debug, Clone)]
pub enum EntityMessage {
    TransformMessage(TransformMessage),
    NameMessage(NameMessage),
    RenderMessage(RenderMessage),
    RigidBodyMessage(RigidBodyMessage),
    PointLightMessage(PointLightMessage),
    EmissiveMessage(EmissiveMessage),
    DirectionalLightMessage(DirectionalLightMessage),
    AmbientMessage(AmbientMessage),
    BillboardMessage(BillboardMessage),
    AddComponent(String),
    AddComponentClicked,
    ToggleEditor,
}

#[derive(Default)]
pub struct EntityEditor {
    entity: SerializedEntity,
    name_editor: Option<NameEditor>,
    transform_editor: Option<TransformEditor>,
    render_editor: Option<RenderEditor>,
    rigid_body_editor: Option<RigidBodyEditor>,
    point_light_editor: Option<PointLightEditor>,
    emissive_editor: Option<EmissiveEditor>,
    ambient_light_editor: Option<AmbientEditor>,
    directional_light_editor: Option<DirectionalLightEditor>,
    billboard_editor: Option<BillboardEditor>,

    show_editors: bool,
    show_editor_state: button::State,

    missing_components: Vec<(String, button::State)>,
    add_component: button::State,
    show_add_components: bool,
}

impl EntityEditor {
    pub fn new(entity: SerializedEntity) -> Self {
        let name_editor = entity.name.as_ref().map(|n| NameEditor::new(n.clone()));
        let transform_editor = entity
            .transform
            .as_ref()
            .map(|t| TransformEditor::new(t.clone()));
        let render_editor = entity.render.as_ref().map(|r| RenderEditor::new(r.clone()));
        let rigid_body_editor = entity
            .rigid_body
            .as_ref()
            .map(|r| RigidBodyEditor::new(r.clone()));
        let emissive_editor = entity.emissive.map(|e| EmissiveEditor::new(e));
        let point_light_editor = entity.point_light.map(|e| PointLightEditor::new(e));
        let directional_light_editor = entity
            .directional_light
            .map(|e| DirectionalLightEditor::new(e));
        let ambient_light_editor = entity.ambient_light.map(|e| AmbientEditor::new(e));
        let billboard_editor = entity
            .billboard
            .as_ref()
            .map(|e| BillboardEditor::new(e.clone()));
        let mut editor = Self {
            entity,
            name_editor,
            transform_editor,
            render_editor,
            rigid_body_editor,
            emissive_editor,
            point_light_editor,
            directional_light_editor,
            ambient_light_editor,
            billboard_editor,
            add_component: button::State::default(),
            show_editors: true,
            show_editor_state: button::State::default(),
            show_add_components: false,
            missing_components: vec![],
        };

        editor.recompute_missing_components();

        editor
    }

    pub fn to_entity(&self) -> SerializedEntity {
        let mut entity = self.entity.clone();

        if let Some(editor) = self.transform_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.name_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.render_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.rigid_body_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.emissive_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.point_light_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.ambient_light_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.directional_light_editor.as_ref() {
            editor.apply(&mut entity);
        }
        if let Some(editor) = self.billboard_editor.as_ref() {
            editor.apply(&mut entity);
        }
        entity
    }

    fn recompute_missing_components(&mut self) {
        let mut missing_components = vec![];

        if self.name_editor.is_none() {
            missing_components.push(("name".to_string(), button::State::default()));
        }
        if self.transform_editor.is_none() {
            missing_components.push(("transform".to_string(), button::State::default()));
        }

        if self.render_editor.is_none() {
            missing_components.push(("render".to_string(), button::State::default()));
        }

        if self.rigid_body_editor.is_none() {
            missing_components.push(("rigid_body".to_string(), button::State::default()));
        }
        if self.point_light_editor.is_none() {
            missing_components.push(("point_light".to_string(), button::State::default()));
        }
        if self.emissive_editor.is_none() {
            missing_components.push(("emissive".to_string(), button::State::default()));
        }
        if self.ambient_light_editor.is_none() {
            missing_components.push(("ambient".to_string(), button::State::default()));
        }
        if self.directional_light_editor.is_none() {
            missing_components.push(("directional_light".to_string(), button::State::default()));
        }
        if self.billboard_editor.is_none() {
            missing_components.push(("billboard".to_string(), button::State::default()));
        }
        self.missing_components = missing_components;
    }
}

impl EditorComponent for EntityEditor {
    type Message = EntityMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            EntityMessage::NameMessage(msg) => {
                if let Some(name_editor) = self.name_editor.as_mut() {
                    name_editor.update(msg).map(EntityMessage::NameMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::TransformMessage(msg) => {
                if let Some(transform_editor) = self.transform_editor.as_mut() {
                    transform_editor
                        .update(msg)
                        .map(EntityMessage::TransformMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::RenderMessage(msg) => {
                if let Some(render_editor) = self.render_editor.as_mut() {
                    render_editor.update(msg).map(EntityMessage::RenderMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::RigidBodyMessage(msg) => {
                if let Some(rigid_body_editor) = self.rigid_body_editor.as_mut() {
                    rigid_body_editor
                        .update(msg)
                        .map(EntityMessage::RigidBodyMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::PointLightMessage(msg) => {
                if let Some(point_light_editor) = self.point_light_editor.as_mut() {
                    point_light_editor
                        .update(msg)
                        .map(EntityMessage::PointLightMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::EmissiveMessage(msg) => {
                if let Some(emissive_editor) = self.emissive_editor.as_mut() {
                    emissive_editor
                        .update(msg)
                        .map(EntityMessage::EmissiveMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::DirectionalLightMessage(msg) => {
                if let Some(directional_light_editor) = self.directional_light_editor.as_mut() {
                    directional_light_editor
                        .update(msg)
                        .map(EntityMessage::DirectionalLightMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::AmbientMessage(msg) => {
                if let Some(ambient_light_editor) = self.ambient_light_editor.as_mut() {
                    ambient_light_editor
                        .update(msg)
                        .map(EntityMessage::AmbientMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::BillboardMessage(msg) => {
                if let Some(billboard_editor) = self.billboard_editor.as_mut() {
                    billboard_editor
                        .update(msg)
                        .map(EntityMessage::BillboardMessage)
                } else {
                    Command::none()
                }
            }
            EntityMessage::AddComponent(component_name) => {
                match component_name.as_str() {
                    "name" => {
                        if self.name_editor.is_none() {
                            self.name_editor = Some(NameEditor::default())
                        }
                    }
                    "transform" => {
                        if self.transform_editor.is_none() {
                            self.transform_editor = Some(TransformEditor::default())
                        }
                    }
                    "render" => {
                        if self.render_editor.is_none() {
                            self.render_editor = Some(RenderEditor::default())
                        }
                    }
                    "rigid_body" => {
                        if self.rigid_body_editor.is_none() {
                            self.rigid_body_editor = Some(RigidBodyEditor::default())
                        }
                    }
                    "point_light" => {
                        if self.point_light_editor.is_none() {
                            self.point_light_editor = Some(PointLightEditor::default())
                        }
                    }
                    "emissive" => {
                        if self.emissive_editor.is_none() {
                            self.emissive_editor = Some(EmissiveEditor::default())
                        }
                    }
                    "ambient" => {
                        if self.ambient_light_editor.is_none() {
                            self.ambient_light_editor = Some(AmbientEditor::default())
                        }
                    }
                    "directional_light" => {
                        if self.directional_light_editor.is_none() {
                            self.directional_light_editor = Some(DirectionalLightEditor::default())
                        }
                    }
                    "billboard" => {
                        if self.billboard_editor.is_none() {
                            self.billboard_editor = Some(BillboardEditor::default())
                        }
                    }
                    _ => (),
                }
                self.recompute_missing_components();
                Command::none()
            }
            EntityMessage::AddComponentClicked => {
                self.show_add_components = !self.show_add_components;
                Command::none()
            }
            EntityMessage::ToggleEditor => {
                self.show_editors = !self.show_editors;
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut col = Column::new().padding(5);

        let header = {
            let label = if self.show_editors { "^" } else { "v" };
            let mut row = Row::new()
                .align_items(Align::Center)
                .push(Text::new("entity").size(10))
                .push(
                    Button::new(&mut self.show_editor_state, Text::new(label).size(10))
                        .on_press(EntityMessage::ToggleEditor),
                );
            row
        };
        col = col.push(header);

        if self.show_editors {
            if let Some(editor) = self.transform_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::TransformMessage(msg)),
                );
            }
            if let Some(editor) = self.name_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::NameMessage(msg)),
                );
            }

            if let Some(editor) = self.render_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::RenderMessage(msg)),
                );
            }

            if let Some(editor) = self.rigid_body_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::RigidBodyMessage(msg)),
                );
            }

            if let Some(editor) = self.ambient_light_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::AmbientMessage(msg)),
                );
            }

            if let Some(editor) = self.directional_light_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::DirectionalLightMessage(msg)),
                );
            }

            if let Some(editor) = self.billboard_editor.as_mut() {
                col = col.push(
                    editor
                        .view()
                        .map(move |msg| EntityMessage::BillboardMessage(msg)),
                );
            }

            col = col.push(
                button::Button::new(&mut self.add_component, Text::new("Add").size(8))
                    .on_press(EntityMessage::AddComponentClicked),
            );
            if self.show_add_components {
                for n in self.missing_components.iter_mut() {
                    col = col.push(
                        button::Button::new(&mut n.1, Text::new(n.0.clone()))
                            .on_press(EntityMessage::AddComponent(n.0.clone())),
                    );
                }
            }
        }

        col.into()
    }

    fn name(&self) -> String {
        String::from("Entity")
    }

    fn apply(&self, entity: &mut SerializedEntity) {}
}

// NAME EDITOR
// =================================================================================================
/// A name is just a string input.
#[derive(Default)]
pub struct NameEditor {
    name: Name,
    state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum NameMessage {
    NameChanged(String),
}

impl NameEditor {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            state: text_input::State::default(),
        }
    }
}

impl EditorComponent for NameEditor {
    type Message = NameMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            NameMessage::NameChanged(new_name) => self.name.0 = new_name,
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .padding(1)
            .spacing(1)
            .push(Text::new("Name").size(10))
            .push(
                TextInput::new(&mut self.state, "", &self.name.0, NameMessage::NameChanged)
                    .padding(1)
                    .size(10),
            )
            .into()
    }

    fn name(&self) -> String {
        String::from("name")
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.name = Some(self.name.clone());
    }
}

// RENDER EDITOR
// =================================================================================================
/// A name is just a string input
#[derive(Default)]
pub struct RenderEditor {
    render: Render,
    state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum RenderMessage {
    ToggleEnable(bool),
    MeshChanged(String),
}

impl RenderEditor {
    pub fn new(render: Render) -> Self {
        Self {
            render,
            state: text_input::State::default(),
        }
    }
}

impl EditorComponent for RenderEditor {
    type Message = RenderMessage;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            RenderMessage::MeshChanged(new_mesh) => self.render.mesh = new_mesh,
            RenderMessage::ToggleEnable(new_enabled) => self.render.enabled = new_enabled,
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("Render").size(10))
            .push(
                TextInput::new(
                    &mut self.state,
                    "",
                    &self.render.mesh,
                    RenderMessage::MeshChanged,
                )
                .padding(1)
                .size(10),
            )
            .push(
                Checkbox::new(self.render.enabled, "enabled?", RenderMessage::ToggleEnable)
                    .size(10),
            )
            .into()
    }

    fn name(&self) -> String {
        String::from("render")
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.render = Some(self.render.clone());
    }
}

// TRANSFORM
// ================================================================================================

pub struct TransformEditor {
    transform: Transform,
    ypr: glam::Vec3,

    translation_editor: FloatVec3Editor,
    scale_editor: FloatVec3Editor,
    rotation_editor: FloatVec3Editor,
}

impl Default for TransformEditor {
    fn default() -> Self {
        Self {
            transform: Transform::default(),
            ypr: glam::Vec3::zero(),
            translation_editor: FloatVec3Editor::default(),
            scale_editor: FloatVec3Editor::default(),
            rotation_editor: FloatVec3Editor::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TransformMessage {
    TranslationChange(FloatVec3Message),
    ScaleChange(FloatVec3Message),
    RotationChanged(FloatVec3Message),
}

impl TransformEditor {
    pub fn new(transform: Transform) -> Self {
        let translation = transform.translation;
        let scale = transform.scale;

        let ypr = quat_to_euler(transform.rotation);
        Self {
            transform,
            ypr,
            translation_editor: FloatVec3Editor::new(translation),
            scale_editor: FloatVec3Editor::new(scale),
            rotation_editor: FloatVec3Editor::new(ypr),
        }
    }

    pub fn get_transform(&self) -> Transform {
        let quat = Quat::from_rotation_ypr(self.ypr.x(), self.ypr.y(), self.ypr.z());
        Transform {
            translation: self.transform.translation,
            scale: self.transform.scale,
            rotation: quat,
        }
    }
}

impl EditorComponent for TransformEditor {
    type Message = TransformMessage;

    fn update(&mut self, message: TransformMessage) -> Command<TransformMessage> {
        use TransformMessage::*;
        match message {
            TranslationChange(msg) => self
                .translation_editor
                .update(msg, &mut self.transform.translation),
            ScaleChange(msg) => self.scale_editor.update(msg, &mut self.transform.scale),
            RotationChanged(msg) => self.rotation_editor.update(msg, &mut self.ypr),
        }
        Command::none()
    }

    fn view(&mut self) -> Element<TransformMessage> {
        Column::new()
            .spacing(1)
            .padding(1)
            .push(Text::new("Transform").size(10))
            .push(
                self.translation_editor
                    .view(self.transform.translation)
                    .map(|e| TransformMessage::TranslationChange(e)),
            )
            .push(
                self.scale_editor
                    .view(self.transform.scale)
                    .map(|e| TransformMessage::TranslationChange(e)),
            )
            .push(
                self.rotation_editor
                    .view(self.ypr)
                    .map(|e| TransformMessage::RotationChanged(e)),
            )
            .into()
    }

    fn name(&self) -> String {
        "transform".to_string()
    }

    fn apply(&self, entity: &mut SerializedEntity) {
        entity.transform = Some(self.get_transform());
    }
}

// ==============================================================================================
#[derive(Debug, Clone)]
pub enum FloatVec3Message {
    XChanged(String),
    YChanged(String),
    ZChanged(String),
    XFinished,
    YFinished,
    ZFinished,
}

#[derive(Default)]
pub struct FloatVec3Editor {
    x_state: text_input::State,
    y_state: text_input::State,
    z_state: text_input::State,

    x_value: String,
    y_value: String,
    z_value: String,

    error_message: Option<String>,
}

impl FloatVec3Editor {
    pub fn new(initial: glam::Vec3) -> Self {
        Self {
            x_state: text_input::State::default(),
            y_state: text_input::State::default(),
            z_state: text_input::State::default(),
            x_value: format!("{}", initial.x()),
            y_value: format!("{}", initial.y()),
            z_value: format!("{}", initial.z()),
            error_message: None,
        }
    }

    pub fn view(&mut self, v: glam::Vec3) -> Element<FloatVec3Message> {
        let x_text_input = TextInput::new(
            &mut self.x_state,
            "X value",
            &self.x_value,
            FloatVec3Message::XChanged,
        )
        .size(10)
        .padding(2)
        .on_submit(FloatVec3Message::XFinished);
        let y_text_input = TextInput::new(
            &mut self.y_state,
            "Y value",
            &self.y_value,
            FloatVec3Message::YChanged,
        )
        .size(10)
        .padding(2)
        .on_submit(FloatVec3Message::YFinished);
        let z_text_input = TextInput::new(
            &mut self.z_state,
            "Z value",
            &self.z_value,
            FloatVec3Message::ZChanged,
        )
        .size(10)
        .padding(2)
        .on_submit(FloatVec3Message::ZFinished);
        let mut col = Column::new().push(
            Row::new()
                .spacing(10)
                .align_items(Align::Center)
                .push(x_text_input)
                .push(y_text_input)
                .push(z_text_input),
        );

        if let Some(e) = self.error_message.as_ref() {
            col = col.push(Text::new(e));
        }
        col.into()
    }

    pub fn update(&mut self, msg: FloatVec3Message, v: &mut glam::Vec3) {
        match msg {
            FloatVec3Message::XChanged(x_value) => {
                self.x_value = x_value;
                match self.x_value.parse::<f32>() {
                    Ok(x_value) => v.set_x(x_value),
                    Err(e) => self.error_message = Some(format!("{}", e)),
                }
            }
            FloatVec3Message::YChanged(y_value) => {
                self.y_value = y_value;
                match self.y_value.parse::<f32>() {
                    Ok(y_state) => v.set_y(y_state),
                    Err(e) => self.error_message = Some(format!("{}", e)),
                }
            }
            FloatVec3Message::ZChanged(z_value) => {
                self.z_value = z_value;
                match self.z_value.parse::<f32>() {
                    Ok(z_value) => v.set_z(z_value),
                    Err(e) => self.error_message = Some(format!("{}", e)),
                }
            }

            FloatVec3Message::XFinished => match self.x_value.parse::<f32>() {
                Ok(x_value) => v.set_x(x_value),
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
            FloatVec3Message::YFinished => match self.y_value.parse::<f32>() {
                Ok(y_state) => v.set_y(y_state),
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
            FloatVec3Message::ZFinished => match self.z_value.parse::<f32>() {
                Ok(z_value) => v.set_z(z_value),
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
        }
    }
}

// ==============================================================================================
#[derive(Debug, Clone)]
pub enum ColorMessage {
    RChanged(String),
    GChanged(String),
    BChanged(String),
}

#[derive(Default)]
pub struct ColorEditor {
    r_state: text_input::State,
    g_state: text_input::State,
    b_state: text_input::State,

    error_message: Option<String>,
}

impl ColorEditor {
    pub fn new() -> Self {
        Self {
            r_state: text_input::State::default(),
            g_state: text_input::State::default(),
            b_state: text_input::State::default(),
            error_message: None,
        }
    }

    pub fn view(&mut self, color: RgbColor) -> Element<ColorMessage> {
        let x_text_input = TextInput::new(
            &mut self.r_state,
            "Red",
            &format!("{}", color.r),
            ColorMessage::RChanged,
        )
        .size(10)
        .padding(2);
        let y_text_input = TextInput::new(
            &mut self.g_state,
            "Green",
            &format!("{}", color.g),
            ColorMessage::GChanged,
        )
        .size(10)
        .padding(2);
        let z_text_input = TextInput::new(
            &mut self.b_state,
            "Blue",
            &format!("{}", color.b),
            ColorMessage::BChanged,
        )
        .size(10)
        .padding(2);
        let mut col = Column::new()
            .push(
                Text::new("Color:")
                    .size(10)
                    .color(iced::Color::from_rgb8(color.r, color.g, color.b)),
            )
            .push(
                Row::new()
                    .spacing(10)
                    .align_items(Align::Center)
                    .push(Text::new("r: ").size(10))
                    .push(x_text_input)
                    .push(Text::new("g: ").size(10))
                    .push(y_text_input)
                    .push(Text::new("b: ").size(10))
                    .push(z_text_input),
            );

        if let Some(e) = self.error_message.as_ref() {
            col = col.push(Text::new(e));
        }
        col.into()
    }

    pub fn update(&mut self, msg: ColorMessage, v: &mut RgbColor) {
        match msg {
            ColorMessage::RChanged(r) => match r.parse::<u8>() {
                Ok(r) => v.r = r,
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
            ColorMessage::GChanged(g) => match g.parse::<u8>() {
                Ok(g) => v.g = g,
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
            ColorMessage::BChanged(b) => match b.parse::<u8>() {
                Ok(b) => v.b = b,
                Err(e) => self.error_message = Some(format!("{}", e)),
            },
        }
    }
}

/**
EulerAngles ToEulerAngles(Quaternion q) {
    EulerAngles angles;

    // roll (x-axis rotation)
    double sinr_cosp = 2 * (q.w * q.x + q.y * q.z);
    double cosr_cosp = 1 - 2 * (q.x * q.x + q.y * q.y);
    angles.roll = std::atan2(sinr_cosp, cosr_cosp);

    // pitch (y-axis rotation)
    double sinp = 2 * (q.w * q.y - q.z * q.x);
    if (std::abs(sinp) >= 1)
        angles.pitch = std::copysign(M_PI / 2, sinp); // use 90 degrees if out of range
    else
        angles.pitch = std::asin(sinp);

    // yaw (z-axis rotation)
    double siny_cosp = 2 * (q.w * q.z + q.x * q.y);
    double cosy_cosp = 1 - 2 * (q.y * q.y + q.z * q.z);
    angles.yaw = std::atan2(siny_cosp, cosy_cosp);

    return angles;
}
**/
fn quat_to_euler(q: Quat) -> glam::Vec3 {
    let (axis, angle) = q.to_axis_angle();
    let rot = UnitQuaternion::from_scaled_axis(
        nalgebra::Vector3::new(axis.x(), axis.y(), axis.z()) * angle,
    );
    let (roll, pitch, yaw) = rot.euler_angles();
    glam::Vec3::new(yaw, pitch, roll)
}
