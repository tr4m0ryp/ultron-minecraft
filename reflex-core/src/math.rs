use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn horizontal(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }
    pub fn dist(self, o: Vec3) -> f64 {
        (self - o).len()
    }
    pub fn normalized(self) -> Vec3 {
        let l = self.len();
        if l < 1e-9 {
            Vec3::ZERO
        } else {
            self * (1.0 / l)
        }
    }
    pub fn block(self) -> BlockPos {
        BlockPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        self * -1.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
    pub fn below(self) -> Self {
        self.offset(0, -1, 0)
    }
    pub fn above(self) -> Self {
        self.offset(0, 1, 0)
    }
    /// Centre of the block's bottom face, where a standing player's feet would be.
    pub fn feet_centre(self) -> Vec3 {
        Vec3::new(self.x as f64 + 0.5, self.y as f64, self.z as f64 + 0.5)
    }
    pub const HORIZONTAL: [(i32, i32); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
}

/// Axis-aligned box in world coordinates, used for inhibition scopes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn contains(&self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }
}

/// Minecraft convention: yaw 0 faces +Z, yaw 90 faces -X; pitch positive looks down.
pub fn yaw_pitch_to(from: Vec3, to: Vec3) -> (f32, f32) {
    let d = to - from;
    let yaw = (-d.x).atan2(d.z).to_degrees();
    let h = (d.x * d.x + d.z * d.z).sqrt();
    let pitch = (-d.y).atan2(h).to_degrees();
    (yaw as f32, pitch as f32)
}

pub fn look_vector(yaw: f32, pitch: f32) -> Vec3 {
    let (y, p) = ((yaw as f64).to_radians(), (pitch as f64).to_radians());
    Vec3::new(-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos())
}

/// Smallest signed difference a - b in degrees, in (-180, 180].
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (a - b) % 360.0;
    if d > 180.0 {
        d -= 360.0;
    } else if d <= -180.0 {
        d += 360.0;
    }
    d
}
