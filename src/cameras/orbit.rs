//! A camera that orbits a target point — the `THREE.OrbitControls` equivalent.

use std::f32::consts::{FRAC_PI_2, PI};

use super::camera::Camera;
use crate::input::{InputState, MouseButton};
use crate::math::{Mat4, Vec3};

/// How many radians the camera turns per pixel of drag. `0.01` is about half a
/// degree, so dragging across a 720px window sweeps just over a full turn.
const ROTATE_PER_PIXEL: f32 = 0.01;

/// Pan distance as a fraction of the orbit distance, per pixel of drag.
const PAN_FRACTION_PER_PIXEL: f32 = 0.002;

/// Scroll sensitivity as a zoom factor per wheel notch (positive scrolls in).
const ZOOM_PER_NOTCH: f32 = 0.1;

/// A pinhole camera that stays pointed at [`target`](OrbitCamera::target) while
/// you drag the mouse — Neptune's `THREE.OrbitControls`.
///
/// Position is tracked in spherical coordinates around the target: an
/// [`azimuth`](OrbitCamera::azimuth) (how far around the world's up axis), a
/// [`polar`](OrbitCamera::polar) angle (how far down from straight above), and
/// a [`distance`](OrbitCamera::distance). It implements [`Camera`] like any
/// other viewpoint, and once per frame you call [`update`](OrbitCamera::update)
/// with the frame's input to apply the standard control scheme:
///
/// - **left-drag** rotates around the target,
/// - **right-drag** pans the target through the scene,
/// - **scroll** dollies in and out.
///
/// The defaults drop the camera at `(0, 0, distance)` looking at the origin,
/// which is where the Three.js starter puts `camera.position.z = 1.5`.
pub struct OrbitCamera {
    /// Vertical field of view, in radians.
    pub fov: f32,
    /// Viewport width divided by height.
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    /// The point the camera circles around.
    pub target: Vec3,
    /// How far around the up axis the camera sits, in radians.
    pub azimuth: f32,
    /// Angle down from the +Y axis, in radians. `π/2` is level with the target.
    pub polar: f32,
    /// How far from the target, in world units.
    pub distance: f32,
    /// Dolly-in limit.
    pub min_distance: f32,
    /// Dolly-out limit.
    pub max_distance: f32,
    /// Closest the camera gets to straight above the target, in radians.
    pub min_polar: f32,
    /// Farthest the camera gets toward straight below the target, in radians.
    pub max_polar: f32,
}

impl OrbitCamera {
    pub fn new(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        OrbitCamera {
            fov,
            aspect,
            near,
            far,
            target: Vec3::ZERO,
            azimuth: 0.0,
            polar: FRAC_PI_2,
            distance: 1.0,
            min_distance: 0.1,
            max_distance: 100.0,
            min_polar: 0.05,
            max_polar: PI - 0.05,
        }
    }

    /// Sets the orbit distance. The Three.js `camera.position.z = 1.5` becomes
    /// `.with_distance(1.5)`, since Neptune builds cameras at the origin.
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    /// Sets the point the camera orbits.
    pub fn with_target(mut self, target: Vec3) -> Self {
        self.target = target;
        self
    }

    /// Where the camera sits, derived from the orbit parameters.
    pub fn position(&self) -> Vec3 {
        let sin_polar = self.polar.sin();
        Vec3::new(
            self.target.x + self.distance * sin_polar * self.azimuth.sin(),
            self.target.y + self.distance * self.polar.cos(),
            self.target.z + self.distance * sin_polar * self.azimuth.cos(),
        )
    }

    /// Turns the camera around the target. `yaw` orbits it around the up axis
    /// (positive moves the camera left, so dragging right spins the object
    /// right under the cursor); `pitch` climbs toward or away from the north
    /// pole, clamped so it never quite flips over the top.
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.azimuth -= yaw;
        self.polar = (self.polar - pitch).clamp(self.min_polar, self.max_polar);
    }

    /// Pans the orbit target in the camera's screen plane: `right` moves the
    /// scene right on screen, `up` moves it up.
    pub fn pan(&mut self, right: f32, up: f32) {
        let forward = (self.target - self.position()).normalize();
        let right_axis = forward.cross(Vec3::Y).normalize();
        let up_axis = right_axis.cross(forward);
        self.target += right_axis * right + up_axis * up;
    }

    /// Dollies the camera toward or away from the target. A `factor` below 1
    /// zooms in, above 1 zooms out; the result is clamped to the distance
    /// limits.
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(self.min_distance, self.max_distance);
    }

    /// Applies the frame's mouse input, `THREE.OrbitControls` style:
    ///
    /// - left-drag rotates around the target,
    /// - right-drag pans the target,
    /// - the scroll wheel dollies in and out.
    ///
    /// Call it once per frame, before rendering.
    pub fn update(&mut self, input: &InputState) {
        let mouse = input.mouse();
        let (dx, dy) = mouse.delta();

        if mouse.held(MouseButton::Left) {
            self.rotate(dx * ROTATE_PER_PIXEL, dy * ROTATE_PER_PIXEL);
        } else if mouse.held(MouseButton::Right) {
            let pan = self.distance * PAN_FRACTION_PER_PIXEL;
            self.pan(dx * pan, -dy * pan);
        }

        let scroll = mouse.scroll();
        if scroll != 0.0 {
            self.zoom((-scroll * ZOOM_PER_NOTCH).exp());
        }
    }
}

impl Camera for OrbitCamera {
    fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;
    use winit::dpi::PhysicalPosition;
    use winit::event::ElementState;

    fn camera() -> OrbitCamera {
        OrbitCamera::new(FRAC_PI_2, 16.0 / 9.0, 0.1, 100.0).with_distance(1.5)
    }

    fn approx(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn the_default_camera_sits_in_front_of_the_target() {
        let cam = camera();
        assert!(approx(cam.position(), Vec3::new(0.0, 0.0, 1.5)));
        assert!(approx(cam.target, Vec3::ZERO));
    }

    #[test]
    fn rotating_yaw_moves_the_camera_around_the_target() {
        let mut cam = camera();
        cam.rotate(FRAC_PI_2, 0.0);
        assert!(approx(cam.position(), Vec3::new(-1.5, 0.0, 0.0)));
        assert!(approx(cam.target, Vec3::ZERO), "orbit keeps the target fixed");
    }

    #[test]
    fn pitching_climbs_toward_the_north_pole() {
        let mut cam = camera();
        cam.rotate(0.0, FRAC_PI_2);
        // polar: π/2 - π/2 = 0 → clamped to min_polar, so the camera sits just
        // short of straight above the target.
        assert!((cam.polar - cam.min_polar).abs() < 1e-6);
        let expected = Vec3::new(
            cam.distance * cam.polar.sin() * cam.azimuth.sin(),
            cam.distance * cam.polar.cos(),
            cam.distance * cam.polar.sin() * cam.azimuth.cos(),
        );
        assert!(approx(cam.position(), expected));
        assert!(cam.position().y > 1.49, "camera climbed toward the pole");
    }

    #[test]
    fn pitching_never_flips_past_a_pole() {
        let mut cam = camera();
        cam.rotate(0.0, 100.0);
        assert!((cam.polar - cam.min_polar).abs() < 1e-6);
        cam.rotate(0.0, -100.0);
        assert!((cam.polar - cam.max_polar).abs() < 1e-6);
    }

    #[test]
    fn the_view_matrix_looks_at_the_target() {
        let cam = camera();
        let seen = cam.view_matrix().transform_point3(cam.target);
        assert!(approx(seen, Vec3::new(0.0, 0.0, -1.5)));
    }

    #[test]
    fn zoom_clamps_to_the_distance_limits() {
        let mut cam = camera();
        cam.zoom(1000.0);
        assert_eq!(cam.distance, cam.max_distance);
        cam.zoom(0.0001);
        assert_eq!(cam.distance, cam.min_distance);
    }

    #[test]
    fn zoom_factor_below_one_moves_the_camera_closer() {
        let mut cam = camera();
        cam.zoom(0.5);
        assert!((cam.distance - 0.75).abs() < 1e-6);
    }

    #[test]
    fn pan_moves_the_target_without_changing_the_orbit() {
        let mut cam = camera();
        cam.pan(2.0, 0.0);
        assert!(approx(cam.target, Vec3::new(2.0, 0.0, 0.0)));
        let offset = cam.position() - cam.target;
        assert!(approx(offset, Vec3::new(0.0, 0.0, 1.5)), "orbit unchanged");
    }

    #[test]
    fn update_rotates_while_the_left_button_is_held() {
        let mut cam = camera();
        let mut input = InputState::new();
        {
            let mouse = input.mouse_mut();
            mouse.handle_cursor_moved(PhysicalPosition::new(100.0, 50.0));
            mouse.handle_button_event(MouseButton::Left, ElementState::Pressed);
            mouse.handle_cursor_moved(PhysicalPosition::new(110.0, 55.0));
        }
        let azimuth = cam.azimuth;
        cam.update(&input);
        assert!(cam.azimuth != azimuth, "dragging turned the camera");
    }

    #[test]
    fn update_zooms_on_scroll() {
        let mut cam = camera();
        let mut input = InputState::new();
        input
            .mouse_mut()
            .handle_scroll(winit::event::MouseScrollDelta::LineDelta(0.0, 1.0));
        let distance = cam.distance;
        cam.update(&input);
        assert!(cam.distance < distance, "scrolling up zoomed in");
    }
}
