use iced::{button, scrollable, Align, Application, Command, Scrollable, Settings};
use iced::{Button, Column, Element, Sandbox, Text};
use r3dtest::ecs::serialization::SerializedEntity;
use r3dtest::editor::{EditorComponent, EntityEditor, EntityMessage};
use std::path::PathBuf;

fn main() {
    Editor::run(Settings::default());
}

// STATE
// -------------------
enum Editor {
    Loading,
    Loaded(EditorState),
    Error(LoadError),
}

pub struct EditorState {
    entities: Vec<EntityEditor>,
    scrollableState: scrollable::State,
    btnState: button::State,
}

impl EditorState {
    pub fn new(entities: Vec<EntityEditor>) -> Self {
        Self {
            entities,
            scrollableState: scrollable::State::default(),
            btnState: button::State::default(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Vec<SerializedEntity>, LoadError>),
    EntityMessage(usize, EntityMessage),
    Save,
    Saved(Result<(), SavedError>),
}

// VIEW
// -----------------------
impl Application for Editor {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Editor::Loading,
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            Editor::Loading => Text::new("Loading").into(),
            Editor::Loaded(ref mut c) => {
                let entities: Element<_> = c
                    .entities
                    .iter_mut()
                    .enumerate()
                    .fold(
                        Scrollable::new(&mut c.scrollableState).spacing(20),
                        |column, (i, entity)| {
                            column.push(
                                entity
                                    .view()
                                    .map(move |message| Message::EntityMessage(i, message)),
                            )
                        },
                    )
                    .into();

                let btn = Button::new(&mut c.btnState, Text::new("Save")).on_press(Message::Save);
                Column::new().padding(10).push(btn).push(entities).into()
            }
            Editor::Error(e) => Text::new(format!("Error = {:?}", e)).into(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(res) => {
                match res {
                    Ok(mut entities) => {
                        *self = Editor::Loaded(EditorState::new(
                            entities.drain(..).map(|c| EntityEditor::new(c)).collect(),
                        ))
                    }
                    Err(e) => *self = Editor::Error(e),
                }

                Command::none()
            }
            Message::EntityMessage(index, msg) => {
                if let Editor::Loaded(ref mut editor) = self {
                    editor.entities[index].update(msg);
                }
                Command::none()
            }
            Message::Save => {
                if let Editor::Loaded(state) = self {
                    let serialized_entity: Vec<_> = state
                        .entities
                        .iter()
                        .map(|editor| editor.to_entity())
                        .collect();
                    Command::perform(save(serialized_entity), Message::Saved)
                } else {
                    Command::none()
                }
            }
            Message::Saved(_) => Command::none(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LoadError {
    FileError,
    FormatError,
    SerError,
}

#[derive(Debug, Clone)]
pub enum SavedError {
    FileError,
    FormatError,
    SerError,
}

struct SavedState {
    state: String,
}

impl SavedState {
    fn path() -> std::path::PathBuf {
        PathBuf::from("./assets/world/lol2.ron")
    }

    async fn load() -> Result<Vec<SerializedEntity>, LoadError> {
        use async_std::prelude::*;

        let mut contents = String::new();

        let mut file = async_std::fs::File::open(Self::path())
            .await
            .map_err(|_| LoadError::FileError)?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|_| LoadError::FileError)?;

        let entities = ron::de::from_str(&contents).map_err(|_| LoadError::SerError)?;
        Ok(entities)
    }
}

async fn save(entities: Vec<SerializedEntity>) -> Result<(), SavedError> {
    let p = PathBuf::from("./assets/world/lol2.ron");
    use async_std::prelude::*;

    let mut contents = String::new();

    let mut file = async_std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(p)
        .await
        .map_err(|_| SavedError::FileError)?;

    let entity_ser = ron::ser::to_string_pretty(&entities, ron::ser::PrettyConfig::default())
        .map_err(|_| SavedError::SerError)?;

    file.write(entity_ser.as_bytes())
        .await
        .map_err(|_| SavedError::FileError)?;

    //let entities = ron::de::from_str(&contents).map_err(|_| LoadError::SerError)?;
    Ok(())
}
