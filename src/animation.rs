use crate::render::billboard::Billboard;
use crate::render::sprite::SpriteRender;
use log::error;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

/// One animation (in one spreadsheet).
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Animation {
    /// Keyframes element are sprite_index and number of frames to elapse for the current
    /// keyframe.
    pub keyframes: Vec<(usize, usize)>,

    pub single: bool,
    /// in frames
    pub current_index: usize,
    // in seconds
    pub elapsed_frame: usize,
}

impl Animation {
    pub fn new(keyframes: Vec<(usize, usize)>) -> Self {
        Self {
            keyframes,
            single: false,
            current_index: 0,
            elapsed_frame: 0,
        }
    }
}

/// All Animations for an entity
/// Control what entity is active with current_animation
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AnimationController {
    /// Animation will cycle through the sprites on its spritesheet
    pub animations: HashMap<String, Animation>,

    /// if set to something, will play the corresponding animation
    pub current_animation: Option<String>,
}

trait Animatable: Send + Sync {
    fn set_animation_frame(&mut self, frame: usize);
}

impl Animatable for SpriteRender {
    fn set_animation_frame(&mut self, frame: usize) {
        self.sprite_nb = frame;
    }
}

impl Animatable for Billboard {
    fn set_animation_frame(&mut self, frame: usize) {
        self.sprite_nb = frame;
    }
}

pub struct AnimationSystem;

impl AnimationSystem {
    pub fn animate(&mut self, world: &mut hecs::World) {
        self.animate_impl::<SpriteRender>(world);
        self.animate_impl::<Billboard>(world);
    }

    fn animate_impl<T>(&mut self, world: &mut hecs::World)
    where
        T: Animatable + 'static,
    {
        for (_, (controller, sprite)) in world.query::<(&mut AnimationController, &mut T)>().iter()
        {
            let mut animation_finished = false;
            if let Some(ref animation_name) = controller.current_animation {
                if let Some(ref mut animation) = controller.animations.get_mut(animation_name) {
                    sprite.set_animation_frame(animation.keyframes[animation.current_index].0);

                    animation.elapsed_frame += 1;
                    if animation.elapsed_frame > animation.keyframes[animation.current_index].1 {
                        animation.elapsed_frame = 0;
                        animation.current_index =
                            (animation.current_index + 1) % animation.keyframes.len();

                        animation_finished = animation.current_index == 0 && animation.single;
                    }
                } else {
                    error!("Cannot find animation with name = {}", animation_name);
                }
            }
            if animation_finished {
                controller.current_animation = None;
            }
        }
    }
}
