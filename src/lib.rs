#[macro_use]
extern crate bitflags;

pub mod animation;
pub mod camera;
pub mod collections;
pub mod colors;
pub mod controller;
pub mod ecs;
pub mod editor;
pub mod event;
pub mod gameplay;
pub mod input;
pub mod net;
pub mod physics;
pub mod render;
pub mod resources;
pub mod scene;
pub mod transform;
pub mod utils;

#[macro_export]
macro_rules! timed {
    ($val:expr) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match std::time::Instant::now() {
            now => match $val {
                tmp => {
                    use log::debug;

                    let elapsed = (std::time::Instant::now() - now).as_millis();
                    debug!(
                        "[{}:{}] {} = {:#?}",
                        file!(),
                        line!(),
                        stringify!($val),
                        elapsed
                    );
                    tmp
                }
            },
        }
    };
}
