use r3dtest::animation::*;
use std::collections::HashMap;

fn main() {
    let animation = Animation {
        current_index: 0,
        elapsed_frame: 0,
        keyframes: vec![(0, 0)],
        single: false,
    };

    let mut animations = HashMap::new();
    animations.insert("h".to_string(), animation);

    let controller = AnimationController {
        current_animation: None,
        animations: animations,
    };

    println!(
        "{:?}",
        ron::ser::to_string_pretty(&controller, ron::ser::PrettyConfig::default())
    );
}
