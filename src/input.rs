use log::{error, info};
use luminance_glfw::{Action, GlfwSurface, Key, MouseButton, Surface, WindowEvent};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Input {
    pub key_down: HashSet<Key>,
    pub mouse_press: HashSet<MouseButton>,
    pub mouse_delta: Option<(f32, f32)>,
    pub mouse_pos: Option<(f32, f32)>,
    pub should_exit: bool,
    pub has_focus: bool,
    // events are what happened during a frame. We just keep interesting events.
    pub events: Vec<WindowEvent>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            has_focus: true,
            ..Self::default()
        }
    }
    pub fn process_events(&mut self, surface: &mut GlfwSurface) {
        self.clear_events();
        for event in surface.poll_events() {
            if let WindowEvent::Focus(has_focus) = event {
                info!(
                    "Window before focus event\nmouse pos = {:?}",
                    self.mouse_pos
                );
                info!("Window focus event = {:?}", has_focus);
                if has_focus {
                    // Has focus = true means that we lost the focus of the window and we regained it.
                    // the previous mouse position is not valid anymore and if we use it it might
                    // cause a big mouse_delta
                    self.mouse_pos = None;
                } else {
                    // it's possible some events have been processed in the event loop before we
                    // actually arrive to the focus event.
                    self.clear_events();
                }
                self.has_focus = has_focus;
                info!("Window after focus event\nmouse pos = {:?}", self.mouse_pos);
            }

            if self.has_focus {
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
                        info!("Cursor pos event; x {} y {}", x, y);
                        let x = x as f32;
                        let y = y as f32;
                        info!("Mouse pos before = {:?}", self.mouse_pos);
                        if let Some((old_x, old_y)) = self.mouse_pos {
                            let mut delta_x = (x - old_x);
                            if delta_x.abs() > 40.0 {
                                delta_x = delta_x.signum() * 40.0;
                            }
                            let mut delta_y = (old_y - y);

                            if delta_y.abs() > 40.0 {
                                delta_y = delta_y.signum() * 40.0;
                            }
                            self.mouse_delta = Some((delta_x, delta_y
//                                (x - old_x) / (x - old_x).abs(),
//                                (old_y - y) / (old_y - y).abs(),
                            ));
                        }

                        self.mouse_pos = Some((x, y));
                        info!(
                            "Mouse pos after: {:?}\nmouse delta after = {:?}",
                            self.mouse_pos, self.mouse_delta
                        );
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
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
        self.mouse_delta = None;
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
