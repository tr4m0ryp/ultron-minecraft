use crate::math::Vec3;

/// Time to contact in ticks: distance over closing speed. `None` if not closing.
pub fn tau(me: Vec3, me_vel: Vec3, other: Vec3, other_vel: Vec3) -> Option<f64> {
    let r = other - me;
    let v = other_vel - me_vel;
    let dist = r.len();
    if dist < 1e-6 {
        return Some(0.0);
    }
    let closing = -r.dot(v) / dist;
    (closing > 1e-4).then(|| dist / closing)
}

/// Closest point of approach of a projectile to a point: (miss distance, ticks until then).
pub fn cpa(target: Vec3, proj: Vec3, proj_vel: Vec3) -> (f64, f64) {
    let r = proj - target;
    let vv = proj_vel.dot(proj_vel);
    if vv < 1e-9 {
        return (r.len(), 0.0);
    }
    let t = (-r.dot(proj_vel) / vv).max(0.0);
    ((r + proj_vel * t).len(), t)
}
