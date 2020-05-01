use nalgebra::UnitQuaternion;

fn from_ypr(yaw: f32, pitch: f32, roll: f32) -> glam::Quat {
    let cy = (yaw * 0.5).cos();
    let sy = (yaw * 0.5).sin();
    let cp = (pitch * 0.5).cos();
    let sp = (pitch * 0.5).sin();
    let cr = (roll * 0.5).cos();
    let sr = (roll * 0.5).sin();

    let w = cr * cp * cy + sr * sp * sy;
    let x = sr * cp * cy - cr * sp * sy;
    let y = cr * sp * cy + sr * cp * sy;
    let z = cr * cp * sy - sr * sp * cy;

    glam::Quat::from_xyzw(x, y, z, w)
}

fn main() {
    let q = from_ypr(1.4, 1.1, 0.3);

    let [qx, qy, qz, qw]: [f32; 4] = q.into();

    // roll (x-axis rotation)
    let sinr_cosp = 2.0 * (qw * qx + qy * qz);
    let cosr_cosp = 1.0 - 2.0 * (qx * qx + qy * qy);
    let roll = sinr_cosp.atan2(cosr_cosp); //std::atan2(sinr_cosp, cosr_cosp);

    println!("Roll {}", roll);

    let sinp = 2.0 * (qw * qy - qz * qx);
    let pitch = if sinp.abs() >= 1.0 {
        sinp.signum() * std::f32::consts::PI
    } else {
        sinp.asin()
    };
    println!("Pitch {}", pitch);

    // yaw (z-axis rotation)
    let siny_cosp = 2.0 * (qw * qz + qx * qy);
    let cosy_cosp = 1.0 - 2.0 * (qy * qy + qz * qz);
    let yaw = siny_cosp.atan2(cosy_cosp);

    println!("Yaw {}", yaw);
}
