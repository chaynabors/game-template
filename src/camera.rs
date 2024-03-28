use glam::{Mat4, Vec3};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    /// The vertical fov of the camera in radians
    pub fov: f32,
    pub near: f32,
}

impl Camera {
    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, Vec3::Y)
    }

    pub fn projection(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(self.fov, aspect_ratio, self.near)
    }

    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        self.projection(aspect_ratio) * self.view()
    }
}
