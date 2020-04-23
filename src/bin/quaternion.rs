use glam::Quat;

fn main() {
    let q = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    println!("{:?}", q);
}
