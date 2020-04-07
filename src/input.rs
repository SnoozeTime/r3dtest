use luminance_glfw::{Action, GlfwSurface, Key, MouseButton, Surface, WindowEvent};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Input {
    pub key_down: HashSet<Key>,
    pub mouse_press: HashSet<MouseButton>,
    pub mouse_delta: Option<(f32, f32)>,
    pub mouse_pos: Option<(f32, f32)>,
    pub should_exit: bool,

    // events are what happened during a frame. We just keep interesting events.
    pub events: Vec<WindowEvent>,
}

impl Input {
    pub fn process_events(&mut self, surface: &mut GlfwSurface) {
        self.events.clear();
        self.mouse_delta = None;
        for event in surface.poll_events() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    self.should_exit = true;
                    break; // stop processing at that point...
                }
                WindowEvent::Key(k, _, Action::Press, _) => {
                    self.key_down.insert(k);
                    self.events.push(event);
                }
                WindowEvent::Key(k, _, Action::Release, _) => {
                    self.key_down.remove(&k);
                }
                WindowEvent::CursorPos(x, y) => {
                    let x = x as f32;
                    let y = y as f32;

                    if let Some((old_x, old_y)) = self.mouse_pos {
                        self.mouse_delta = Some((x - old_x, old_y - y));
                    }

                    self.mouse_pos = Some((x, y));
                }
                WindowEvent::MouseButton(button, Action::Press, _) => {
                    self.mouse_press.insert(button);
                    self.events.push(event);
                }
                WindowEvent::MouseButton(button, Action::Release, _) => {
                    self.mouse_press.remove(&button);
                }
                _ => (),
            }
        }
    }

    pub fn has_key_down(&self, key: Key) -> bool {
        self.key_down.contains(&key)
    }

    pub fn is_mouse_down(&self, btn: MouseButton) -> bool {
        self.mouse_press.contains(&btn)
    }

    pub fn has_mouse_event_happened(&self, btn: MouseButton, action: Action) -> bool {
        for ev in &self.events {
            match ev {
                WindowEvent::MouseButton(the_btn, the_action, _)
                    if btn == *the_btn && action == *the_action =>
                {
                    return true
                }
                _ => (),
            }
        }
        false
    }

    pub fn has_key_event_happened(&self, key: Key, action: Action) -> bool {
        for ev in &self.events {
            match ev {
                WindowEvent::Key(the_k, _, the_action, _)
                    if key == *the_k && action == *the_action =>
                {
                    return true
                }
                _ => (),
            }
        }
        false
    }
}
