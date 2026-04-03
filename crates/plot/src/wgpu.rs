use super::{Paint, Plotter};
use font::OSIFONT;
use types::{
    constants::FONT_SCALING, gr::{Effects, Pos, Pt, Pts, Rect}
};
use glam::DVec2;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

use ::wgpu::util::DeviceExt;
pub use egui_winit::egui;

#[cfg(not(target_arch = "wasm32"))]
use super::egui_utils;

// New import for fontdue
use fontdue::{Font, FontSettings};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct QuadVertex {
    position: [f32; 2],
}

const QUAD_VERTICES: &[QuadVertex] = &[
    QuadVertex {
        position: [-0.5, -0.5],
    },
    QuadVertex {
        position: [0.5, -0.5],
    },
    QuadVertex {
        position: [-0.5, 0.5],
    },
    QuadVertex {
        position: [0.5, 0.5],
    },
];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    screen_size: [f32; 2],
    _padding: [f32; 2],
}

// Updated struct layout for 64-byte alignment and safer padding
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveInstance {
    p1: [f32; 2],    // 0-8    : Line Start / Circle Center
    p2: [f32; 2],    // 8-16   : Line End   / Text Size
    p3: [f32; 2],    // 16-24  : UV Origin
    _pad1: [f32; 2], // 24-32  : Padding to align `color` to 32
    color: [f32; 4], // 32-48  : RGBA (16-byte aligned)
    width: f32,      // 48-52  : Stroke width
    radius: f32,     // 52-56  : Circle radius
    type_id: u32,    // 56-60  : Type
    angle: f32,      // 60-64  : Rotation (radians)
}

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    // UV coordinates in the texture (0.0 to 1.0)
    pub uv_rect: [f64; 4], // [u_min, v_min, u_width, v_height]
    // Physical size of the character in world units (relative to font scale)
    pub size: [f32; 2],
    // Offset from the cursor position to the top-left of the glyph
    pub bearing: [f32; 2],
    // How far to move the cursor after this character
    pub advance: f32,
}

pub struct WgpuPlotter {
    viewbox: Option<Rect>,
    scale: f64,

    data: Vec<PrimitiveInstance>,
    pending_text: Vec<(String, Pos, Effects)>,
    pending_indices: Vec<usize>,
    current_pos: Pt,

    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    state: Option<State>,
    user_zoom: f64,
    user_pan: Pt,
    cursor_pos: Pt,
    is_dragging: bool,
    quit_requested: bool,

    #[allow(clippy::type_complexity)]
    ui_callback: Option<Box<dyn FnMut(&mut egui::Ui) -> bool>>,
    #[allow(clippy::type_complexity)]
    replot_callback: Option<Box<dyn FnMut(&mut Self)>>,
    pages: Vec<(String, String)>,
    active_page: usize,
}

#[allow(clippy::new_without_default)]
impl WgpuPlotter {
    pub fn new(event_loop: &EventLoop<UserEvent>) -> Self {
        let proxy = Some(event_loop.create_proxy());
        WgpuPlotter {
            state: None,
            proxy,
            viewbox: None,
            scale: 1.0,
            data: Vec::new(),
            pending_text: Vec::new(),
            pending_indices: Vec::new(),
            current_pos: Pt { x: 0.0, y: 0.0 },
            user_zoom: 1.0,
            user_pan: Pt { x: 0.0, y: 0.0 },
            cursor_pos: Pt { x: 0.0, y: 0.0 },
            is_dragging: false,
            quit_requested: false,
            ui_callback: None,
            replot_callback: None,
            pages: Vec::new(),
            active_page: 0,
        }
    }

    pub fn set_active_page(&mut self, page: usize) {
        if page < self.pages.len() {
            self.active_page = page;
        }
    }

    pub fn active_page(&self) -> usize {
        self.active_page
    }

    pub fn pages(&self) -> &[(String, String)] {
        &self.pages
    }

    pub fn set_ui_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut egui::Ui) -> bool + 'static,
    {
        self.ui_callback = Some(Box::new(callback));
    }

    pub fn set_replot_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Self) + 'static,
    {
        self.replot_callback = Some(Box::new(callback));
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.pending_indices.clear();
        self.pending_text.clear();
        self.viewbox = None;
    }

    fn clean_points(&self, indices: &[usize]) -> Vec<usize> {
        if indices.len() < 3 {
            return indices.to_vec();
        }

        let mut current_indices = indices.to_vec();
        let mut changed = true;

        while changed {
            changed = false;
            if current_indices.len() < 3 {
                break;
            }
            let mut next_pass = Vec::new();
            let len = current_indices.len();

            for i in 0..len {
                let prev = current_indices[(i + len - 1) % len];
                let curr = current_indices[i];
                let next = current_indices[(i + 1) % len];

                let p_prev = DVec2::new(self.data[prev].p1[0] as f64, self.data[prev].p1[1] as f64);
                let p_curr = DVec2::new(self.data[curr].p1[0] as f64, self.data[curr].p1[1] as f64);
                let p_next = DVec2::new(self.data[next].p1[0] as f64, self.data[next].p1[1] as f64);

                // 1. Remove Duplicate (On top of previous)
                if (p_curr - p_prev).length() < 0.001 {
                    changed = true;
                    continue;
                }

                // 2. Remove Spike (Antenna)
                if (p_prev - p_next).length() < 0.001 {
                    changed = true;
                    continue;
                }

                // 3. Remove Collinear
                let v1 = p_curr - p_prev;
                let v2 = p_next - p_curr;
                if (self.cross(v1, v2).abs() < 0.001) && (v1.dot(v2) > 0.0) {
                    changed = true;
                    continue;
                }

                next_pass.push(curr);
            }
            current_indices = next_pass;
        }
        current_indices
    }

    fn get_signed_area(&self, indices: &[usize]) -> f32 {
        let mut sum = 0.0;
        for i in 0..indices.len() {
            let p1 = self.data[indices[i]].p1;
            let p2 = self.data[indices[(i + 1) % indices.len()]].p1;
            sum += (p2[0] - p1[0]) * (p2[1] + p1[1]);
        }
        sum
    }

    fn cross(&self, a: DVec2, b: DVec2) -> f64 {
        a.x * b.y - a.y * b.x
    }

    fn get_item(&self, index: isize, list: &[usize]) -> usize {
        if index >= list.len() as isize {
            list[(index % list.len() as isize) as usize]
        } else if index < 0 {
            list[((index % list.len() as isize) + list.len() as isize) as usize]
        } else {
            list[index as usize]
        }
    }

    fn in_triangle(&self, p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
        let v0 = c - a;
        let v1 = b - a;
        let v2 = p - a;

        let dot00 = v0.x * v0.x + v0.y * v0.y;
        let dot01 = v0.x * v1.x + v0.y * v1.y;
        let dot02 = v0.x * v2.x + v0.y * v2.y;
        let dot11 = v1.x * v1.x + v1.y * v1.y;
        let dot12 = v1.x * v2.x + v1.y * v2.y;

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        let epsilon = 0.0000001;
        (u > epsilon) && (v > epsilon) && (u + v < 1.0 - epsilon)
    }

    fn triangulate(&mut self, index_list: &[usize]) -> Vec<[DVec2; 3]> {
        if self.data.len() < 3 {
            return vec![];
        }

        let mut index_list = self.clean_points(index_list);
        let area = self.get_signed_area(&index_list);
        let should_be_positive = area > 0.0;

        let mut total_triangles: Vec<[DVec2; 3]> = vec![];

        let mut safety_count = 0;
        let max_iterations = self.data.len() * 3;

        while index_list.len() > 3 {
            if safety_count > max_iterations {
                spdlog::error!("Max iterations reached. Polygon might be self-intersecting.");
                break;
            }

            let mut ear_found = false;

            for i in 0..index_list.len() {
                let a = self.get_item(i as isize, &index_list); // Tip
                let b = self.get_item(i as isize - 1, &index_list); // Prev
                let c = self.get_item(i as isize + 1, &index_list); // Next

                let va = DVec2::new(self.data[a].p1[0] as f64, self.data[a].p1[1] as f64);
                let vb = DVec2::new(self.data[b].p1[0] as f64, self.data[b].p1[1] as f64);
                let vc = DVec2::new(self.data[c].p1[0] as f64, self.data[c].p1[1] as f64);

                let vab = vb - va;
                let vac = vc - va;

                let cp = self.cross(vab, vac);

                let is_convex = if should_be_positive {
                    cp > 0.0
                } else {
                    cp < 0.0
                };

                if !is_convex {
                    continue;
                }

                let mut is_ear = true;
                for other_idx in &index_list {
                    if *other_idx == a || *other_idx == b || *other_idx == c {
                        continue;
                    }

                    let other_point = DVec2::new(
                        self.data[*other_idx].p1[0] as f64,
                        self.data[*other_idx].p1[1] as f64,
                    );

                    if self.in_triangle(other_point, vb, va, vc) {
                        is_ear = false;
                        break;
                    }
                }

                if is_ear {
                    total_triangles.push([vb, va, vc]);
                    index_list.remove(i);
                    ear_found = true;
                    break;
                }
            }

            if !ear_found {
                break;
            }

            safety_count += 1;
        }

        if index_list.len() == 3 {
            total_triangles.push([
                DVec2::new(
                    self.data[index_list[0]].p1[0] as f64,
                    self.data[index_list[0]].p1[1] as f64,
                ),
                DVec2::new(
                    self.data[index_list[1]].p1[0] as f64,
                    self.data[index_list[1]].p1[1] as f64,
                ),
                DVec2::new(
                    self.data[index_list[2]].p1[0] as f64,
                    self.data[index_list[2]].p1[1] as f64,
                ),
            ]);
        }

        total_triangles
    }

    // Process text that was requested before GPU init
    fn process_pending_text(&mut self) {
        if self.state.is_none() || self.pending_text.is_empty() {
            return;
        }

        let pending = std::mem::take(&mut self.pending_text);
        for (text, pos, effects) in pending {
            self.text(&text, pos, effects);
        }
    }
}

impl Plotter for WgpuPlotter {
    fn open(&self) {}
    fn save(self, _path: &std::path::Path) -> std::io::Result<()> {
        Ok(())
    }
    fn write<W: Write>(self, _writer: &mut W) -> std::io::Result<(u32, u32)> {
        Ok((0, 0))
    }
    fn set_view_box(&mut self, rect: Rect) {
        self.viewbox = Some(rect)
    }
    fn scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    fn move_to(&mut self, pt: Pt) {
        self.current_pos = pt;
    }

    fn line_to(&mut self, pt: Pt) {
        let instance = PrimitiveInstance {
            p1: [self.current_pos.x as f32, self.current_pos.y as f32],
            p2: [pt.x as f32, pt.y as f32],
            p3: [0.0, 0.0],
            _pad1: [0.0; 2],
            color: [1.0, 1.0, 1.0, 1.0], // Default color
            width: 1.0,
            radius: 0.0,
            type_id: 0, // 0 = Line
            angle: 0.0,
        };
        self.data.push(instance);
        self.pending_indices.push(self.data.len() - 1);
        self.current_pos = pt;
    }

    fn close(&mut self) {
        // log::debug!("close is not implemented");
    }

    fn stroke(&mut self, stroke: Paint) {
        if let Some(fill_color) = stroke.fill {
            let mut index_list = self.pending_indices.clone();
            if let Some(&last_idx) = index_list.last() {
                if let Some(&first_idx) = index_list.first() {
                    let last_seg_end = self.data[last_idx].p2;
                    let first_seg_start = self.data[first_idx].p1;

                    let end = DVec2::new(last_seg_end[0] as f64, last_seg_end[1] as f64);
                    let start = DVec2::new(first_seg_start[0] as f64, first_seg_start[1] as f64);

                    if (end - start).length() > 0.001 {
                        self.data.push(PrimitiveInstance {
                            p1: last_seg_end,
                            p2: last_seg_end,
                            p3: [0.0, 0.0],
                            _pad1: [0.0; 2],
                            color: stroke.color.into(),
                            width: stroke.width as f32,
                            radius: 0.0,
                            type_id: 0,
                            angle: 0.0,
                        });
                        index_list.push(self.data.len() - 1);
                    }
                }
            }

            let triangles = self.triangulate(&index_list);
            for tri in triangles {
                self.data.push(PrimitiveInstance {
                    p1: [tri[0].x as f32, tri[0].y as f32],
                    p2: [tri[1].x as f32, tri[1].y as f32],
                    p3: [tri[2].x as f32, tri[2].y as f32],
                    _pad1: [0.0; 2],
                    color: fill_color.into(),
                    width: 0.0,
                    radius: 0.0,
                    type_id: 3,
                    angle: 0.0,
                });
            }
        }

        for idx in self.pending_indices.drain(..) {
            if let Some(instance) = self.data.get_mut(idx) {
                instance.width = stroke.width as f32;
                instance.color = stroke.color.into();
            }
        }
    }

    fn rect(&mut self, rect: Rect, stroke: Paint) {
        let end_x = rect.start.x + rect.end.x;
        let end_y = rect.start.y + rect.end.y;
        self.move_to(rect.start);
        self.line_to(Pt {
            x: end_x,
            y: rect.start.y,
        });
        self.line_to(Pt { x: end_x, y: end_y });
        self.line_to(Pt {
            x: rect.start.x,
            y: end_y,
        });
        self.line_to(rect.start);
        self.stroke(stroke);
    }

    fn arc(&mut self, start: Pt, mid: Pt, end: Pt, stroke: Paint) {
        let p1 = DVec2::new(start.x, start.y);
        let p2 = DVec2::new(mid.x, mid.y);
        let p3 = DVec2::new(end.x, end.y);

        let det = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x);

        if det.abs() < 1e-6 {
            // Collinear: draw straight lines
            self.move_to(start);
            self.line_to(mid);
            self.line_to(end);
            self.stroke(stroke);
            return;
        }

        // Center calculation
        let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
        let ux = ((p1.x.powi(2) + p1.y.powi(2)) * (p2.y - p3.y)
            + (p2.x.powi(2) + p2.y.powi(2)) * (p3.y - p1.y)
            + (p3.x.powi(2) + p3.y.powi(2)) * (p1.y - p2.y))
            / d;
        let uy = ((p1.x.powi(2) + p1.y.powi(2)) * (p3.x - p2.x)
            + (p2.x.powi(2) + p2.y.powi(2)) * (p1.x - p3.x)
            + (p3.x.powi(2) + p3.y.powi(2)) * (p2.x - p1.x))
            / d;

        let center = DVec2::new(ux, uy);
        let radius = (p1 - center).length();

        let angle_start = (p1.y - center.y).atan2(p1.x - center.x);
        let angle_mid = (p2.y - center.y).atan2(p2.x - center.x);
        let angle_end = (p3.y - center.y).atan2(p3.x - center.x);

        // Normalize angle difference to [-PI, PI]
        let normalize = |a: f64| -> f64 {
            let mut a = a;
            while a <= -std::f64::consts::PI {
                a += 2.0 * std::f64::consts::PI;
            }
            while a > std::f64::consts::PI {
                a -= 2.0 * std::f64::consts::PI;
            }
            a
        };

        let sweep1 = normalize(angle_mid - angle_start);
        let sweep2 = normalize(angle_end - angle_mid);

        // Segment 1: Start -> Mid
        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [angle_start as f32, sweep1 as f32],
            p3: [0.0, 0.0],
            _pad1: [0.0; 2],
            color: stroke.color.into(),
            width: stroke.width as f32,
            radius: radius as f32,
            type_id: 5, // Arc
            angle: 0.0,
        });

        // Segment 2: Mid -> End
        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [angle_mid as f32, sweep2 as f32],
            p3: [0.0, 0.0],
            _pad1: [0.0; 2],
            color: stroke.color.into(),
            width: stroke.width as f32,
            radius: radius as f32,
            type_id: 5, // Arc
            angle: 0.0,
        });

        self.current_pos = end;
    }

    fn circle(&mut self, center: Pt, radius: f64, stroke: Paint) {
        let is_filled = stroke.fill.is_some();

        let color: [f32; 4] = if is_filled {
            stroke.fill.unwrap().into()
        } else {
            stroke.color.into()
        };

        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [0.0, 0.0],
            p3: [0.0, 0.0],
            _pad1: [0.0; 2],
            color,
            width: stroke.width as f32,
            radius: radius as f32,
            type_id: if is_filled { 2 } else { 1 },
            angle: 0.0,
        });
    }

    fn polyline(&mut self, pts: Pts, stroke: Paint) {
        if let Some(first) = pts.0.first() {
            self.move_to(*first);
            pts.0.iter().skip(1).for_each(|p| self.line_to(*p));
        }
        self.stroke(stroke);
    }

    fn text(&mut self, text: &str, pos: Pos, effects: Effects) {
        // If State isn't ready, buffer the text call
        let state = match &self.state {
            Some(s) => s,
            None => {
                self.pending_text.push((text.to_string(), pos, effects));
                return;
            }
        };

        let font_size_world = effects.font.size.0 * FONT_SCALING;
        let font_scale = font_size_world / 64.0;

        // Convert degrees to radians
        let angle_rad = pos.angle.to_radians();

        // Rotation helpers
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let rotate =
            |x: f64, y: f64| -> (f64, f64) { (x * cos_a - y * sin_a, x * sin_a + y * cos_a) };

        // Start drawing exactly at pos
        let mut cursor_x = pos.x;
        let mut cursor_y = pos.y;

        for c in text.chars() {
            if let Some(info) = state.font_lookup.get(&c) {
                let width =  (info.size[0] * font_scale) as f64;
                let height = (info.size[1] * font_scale) as f64;

                // Calculate the offset from cursor to the CENTER of the glyph
                let bearing_x = (info.bearing[0] * font_scale) as f64;
                let bearing_y = (info.bearing[1] * font_scale) as f64;

                let center_offset_x = bearing_x + (width / 2.0);
                let center_offset_y = bearing_y + (height / 2.0);

                // Rotate the center offset relative to the cursor
                let (rot_cx, rot_cy) = rotate(center_offset_x, center_offset_y);

                // Final absolute center position
                let final_x = cursor_x + rot_cx;
                let final_y = cursor_y + rot_cy;

                if width > 0.0 {
                    self.data.push(PrimitiveInstance {
                        p1: [final_x as f32, final_y as f32], // Pass CENTER, not Top-Left
                        p2: [width as f32, height as f32],
                        p3: [info.uv_rect[0] as f32, info.uv_rect[1] as f32], // UV Top-Left
                        _pad1: [0.0; 2],
                        color: effects.font.color.unwrap_or_default().into(),
                        width: info.uv_rect[2] as f32,  // UV Width
                        radius: info.uv_rect[3] as f32, // UV Height
                        type_id: 4,
                        angle: angle_rad as f32,
                    });
                }

                // Advance cursor along the rotation vector
                let (adv_x, adv_y) = rotate(info.advance as f64 * font_scale as f64, 0.0);
                cursor_x += adv_x;
                cursor_y += adv_y;
            }
        }
    }

    fn set_pages(&mut self, pages: Vec<(String, String)>) {
        self.pages = pages;
        if self.active_page >= self.pages.len() && !self.pages.is_empty() {
            self.active_page = 0;
        }
    }
}

pub struct State {
    window: Arc<Window>,
    device: ::wgpu::Device,
    queue: ::wgpu::Queue,
    surface: ::wgpu::Surface<'static>,
    config: ::wgpu::SurfaceConfiguration,
    pub render_pipeline: ::wgpu::RenderPipeline,
    pub font_lookup: HashMap<char, GlyphInfo>,

    uniform_buffer: ::wgpu::Buffer,
    uniform_bind_group: ::wgpu::BindGroup,
    quad_vertex_buffer: ::wgpu::Buffer,
    instance_buffer: ::wgpu::Buffer,
    instance_capacity: usize,
    diffuse_bind_group: ::wgpu::BindGroup,

    #[cfg(not(target_arch = "wasm32"))]
    egui_renderer: egui_utils::EguiRenderer,
}

impl State {
    async fn new(
        window: Arc<Window>,
        #[cfg(not(target_arch = "wasm32"))] _proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    ) -> Result<State, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        let instance = ::wgpu::Instance::new(&::wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&::wgpu::RequestAdapterOptions {
                power_preference: ::wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&::wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: ::wgpu::Features::default(),
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = ::wgpu::SurfaceConfiguration {
            usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: ::wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ::wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // --- BUFFERS ---
        let quad_vertex_buffer = device.create_buffer_init(&::wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: ::wgpu::BufferUsages::VERTEX,
        });

        let initial_instances = 100;
        let instance_buffer_size =
            (initial_instances * std::mem::size_of::<PrimitiveInstance>()) as u64;
        let instance_buffer = device.create_buffer(&::wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: instance_buffer_size,
            usage: ::wgpu::BufferUsages::VERTEX | ::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_uniform = CameraUniform {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            screen_size: [size.width as f32, size.height as f32],
            _padding: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&::wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: ::wgpu::BufferUsages::UNIFORM | ::wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ::wgpu::ShaderStages::VERTEX,
                    ty: ::wgpu::BindingType::Buffer {
                        ty: ::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&::wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[::wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // --- TEXTURES ---
        let texture_bind_group_layout =
            device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    ::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ::wgpu::ShaderStages::FRAGMENT,
                        ty: ::wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: ::wgpu::TextureViewDimension::D2,
                            sample_type: ::wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    ::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ::wgpu::ShaderStages::FRAGMENT,
                        ty: ::wgpu::BindingType::Sampler(::wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // --- PIPELINE ---
        let render_pipeline_layout =
            device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: ::wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    ::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<QuadVertex>() as u64,
                        step_mode: ::wgpu::VertexStepMode::Vertex,
                        attributes: &[::wgpu::VertexAttribute {
                            format: ::wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        }],
                    },
                    ::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<PrimitiveInstance>() as u64,
                        step_mode: ::wgpu::VertexStepMode::Instance,
                        attributes: &[
                            ::wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 5,
                                format: ::wgpu::VertexFormat::Float32x2,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 6,
                                format: ::wgpu::VertexFormat::Float32x2,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 16,
                                shader_location: 7,
                                format: ::wgpu::VertexFormat::Float32x2,
                            },
                            // Offset 24-32 is padding, skip.
                            ::wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 8,
                                format: ::wgpu::VertexFormat::Float32x4,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 9,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 52,
                                shader_location: 10,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 56,
                                shader_location: 11,
                                format: ::wgpu::VertexFormat::Uint32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 60,
                                shader_location: 12,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                        ],
                    },
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(::wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(::wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(::wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ::wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: ::wgpu::PrimitiveState {
                topology: ::wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: ::wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        #[cfg(not(target_arch = "wasm32"))]
        let egui_renderer = egui_utils::EguiRenderer::new(&device, config.format, &window);

        // --- FONT LOADING ---
        // Try Linux path, then Windows path, then fallback to embedded OSIFONT
        let font_bytes = if std::path::Path::new("/usr/share/fonts/TTF/DejaVuSansMono.ttf").exists()
        {
            std::fs::read("/usr/share/fonts/TTF/DejaVuSansMono.ttf").unwrap()
        } else if std::path::Path::new("C:\\Windows\\Fonts\\consola.ttf").exists() {
            std::fs::read("C:\\Windows\\Fonts\\consola.ttf").unwrap()
        } else {
            OSIFONT.to_vec()
        };

        // Initialize Fontdue
        let font = Font::from_bytes(font_bytes, FontSettings::default()).unwrap();
        let font_size = 64.0;

        let mut font_lookup = HashMap::new();
        let mut atlas_data = vec![0u8; 512 * 512];
        let atlas_w = 512;
        let atlas_h = 512;
        let mut cx = 0;
        let mut cy = 0;
        let pad = 2;

        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,!?:;+-=()[]{} \"'/_%@#";

        for ch in chars.chars() {
            let (metrics, bitmap) = font.rasterize(ch, font_size);

            if metrics.width > 0 {
                let w = metrics.width as u32;
                let h = metrics.height as u32;

                if cx + w + pad > atlas_w {
                    cx = 0;
                    cy += 64 + pad;
                }

                // Copy bitmap to atlas
                // Fontdue bitmaps are row-major byte vectors
                for (i, v) in bitmap.iter().enumerate() {
                    let x = (i % metrics.width) as u32;
                    let y = (i / metrics.width) as u32;
                    let idx = ((cy + y) * atlas_w + (cx + x)) as usize;
                    if idx < atlas_data.len() {
                        atlas_data[idx] = *v;
                    }
                }

                font_lookup.insert(
                    ch,
                    GlyphInfo {
                        uv_rect: [
                            cx as f64 / atlas_w as f64,
                            cy as f64 / atlas_h as f64,
                            w as f64 / atlas_w as f64,
                            h as f64 / atlas_h as f64,
                        ],
                        size: [w as f32, h as f32],
                        // Transform bearings to Top-Left relative to Baseline (Y-Down screen coords)
                        // metrics.xmin is Left Bearing
                        // metrics.ymin is Bottom Bearing relative to Baseline (Y-Up)
                        // Top in Y-Up is ymin + height
                        // Top in Y-Down is -(ymin + height)
                        bearing: [
                            metrics.xmin as f32,
                            -(metrics.ymin as f32 + metrics.height as f32),
                        ],
                        advance: metrics.advance_width,
                    },
                );

                cx += w + pad;
            } else {
                font_lookup.insert(
                    ch,
                    GlyphInfo {
                        uv_rect: [0.0, 0.0, 0.0, 0.0],
                        size: [0.0, 0.0],
                        bearing: [0.0, 0.0],
                        advance: metrics.advance_width,
                    },
                );
            }
        }

        let diffuse_texture = device.create_texture_with_data(
            &queue,
            &::wgpu::TextureDescriptor {
                size: ::wgpu::Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: ::wgpu::TextureDimension::D2,
                format: ::wgpu::TextureFormat::R8Unorm,
                usage: ::wgpu::TextureUsages::TEXTURE_BINDING | ::wgpu::TextureUsages::COPY_DST,
                label: Some("SDF Texture"),
                view_formats: &[],
            },
            ::wgpu::util::TextureDataOrder::LayerMajor,
            &atlas_data,
        );

        let diffuse_texture_view =
            diffuse_texture.create_view(&::wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&::wgpu::SamplerDescriptor {
            address_mode_u: ::wgpu::AddressMode::ClampToEdge,
            address_mode_v: ::wgpu::AddressMode::ClampToEdge,
            mag_filter: ::wgpu::FilterMode::Linear,
            min_filter: ::wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let diffuse_bind_group = device.create_bind_group(&::wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                ::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ::wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                ::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ::wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: initial_instances,
            diffuse_bind_group,
            #[cfg(not(target_arch = "wasm32"))]
            egui_renderer,
            font_lookup,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn wait_for_idle(&self) {
        self.device
            .poll(::wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();
    }

    fn render(
        &mut self,
        primitives: &[PrimitiveInstance],
        viewbox: Option<Rect>,
        user_zoom: f64,
        user_pan: Pt,
        ui_callback: impl FnOnce(&egui::Context) -> bool,
    ) -> Result<bool, ::wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let replot;
        {
            let view = output
                .texture
                .create_view(&::wgpu::TextureViewDescriptor::default());

            let mut encoder =
                self.device
                    .create_command_encoder(&::wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            #[cfg(not(target_arch = "wasm32"))]
            {
                self.egui_renderer.begin_frame(&self.window);
                replot = ui_callback(self.egui_renderer.context());
            }

            let needed_size = std::mem::size_of_val(primitives) as u64;
            if needed_size > self.instance_buffer.size() {
                self.instance_buffer.destroy();
                self.instance_buffer = self.device.create_buffer(&::wgpu::BufferDescriptor {
                    label: Some("Instance Buffer Resize"),
                    size: needed_size * 2,
                    usage: ::wgpu::BufferUsages::VERTEX | ::wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.instance_capacity = primitives.len() * 2;
            }

            if !primitives.is_empty() {
                self.queue
                    .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(primitives));
            }

            let width = self.config.width as f32;
            let height = self.config.height as f32;

            let (min_x, min_y, data_w, data_h) = if let Some(vb) = viewbox {
                (vb.start.x as f32, vb.start.y as f32, vb.end.x as f32, vb.end.y as f32)
            } else {
                (0.0, 0.0, width, height)
            };

            let data_center_x = min_x + data_w / 2.0;
            let data_center_y = min_y + data_h / 2.0;

            let fit_scale_x = width / data_w;
            let fit_scale_y = height / data_h;
            let base_scale = fit_scale_x.min(fit_scale_y) * 0.95;
            let base_scale = if base_scale.is_normal() {
                base_scale
            } else {
                1.0
            };

            let final_scale = base_scale * user_zoom as f32;

            // Screen Mapping
            let ndc_rx = 2.0 / width;
            let ndc_ry = 2.0 / height;

            let sx = final_scale * ndc_rx;
            let sy = final_scale * -ndc_ry; // Flip Y for Y-Down coordinate systems

            let tx = (-data_center_x * sx) + (user_pan.x as f32 * ndc_rx);
            let ty = (-data_center_y * sy) + (user_pan.y as f32 * -ndc_ry);

            let ortho = [
                [sx, 0.0, 0.0, 0.0],
                [0.0, sy, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [tx, ty, 0.0, 1.0],
            ];

            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[CameraUniform {
                    view_proj: ortho,
                    screen_size: [width, height],
                    _padding: [0.0; 2],
                }]),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: ::wgpu::Operations {
                            load: ::wgpu::LoadOp::Clear(::wgpu::Color::BLACK),
                            store: ::wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &self.diffuse_bind_group, &[]);
                if !primitives.is_empty() {
                    render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
                    render_pass.set_vertex_buffer(1, self.instance_buffer.slice(0..needed_size));
                    render_pass.draw(0..4, 0..primitives.len() as u32);
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let screen_descriptor = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [self.config.width, self.config.height],
                    pixels_per_point: self.window.scale_factor() as f32,
                };
                self.egui_renderer.end_frame_and_draw(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &self.window,
                    &view,
                    screen_descriptor,
                );
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }
        output.present();

        Ok(replot)
    }
}

#[allow(clippy::large_enum_variant)]
pub enum UserEvent {
    StateInitialized(State),
    GenerateMaze,
    SolveMaze,
    Generator(u8),
    Solver(u8),
    Size(usize),
    StepsPerFrame(usize),
    ThemeChanged,
}

impl std::fmt::Debug for UserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserEvent")
    }
}

impl ApplicationHandler<UserEvent> for WgpuPlotter {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(proxy) = self.proxy.take() {
                self.state = Some(pollster::block_on(State::new(window, proxy)).unwrap());
            }
        }

        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        if let UserEvent::StateInitialized(initial_state) = event {
            self.state = Some(initial_state);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        let mut egui_consumed = false;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(state) = &mut self.state {
            let response = state.egui_renderer.handle_input(&state.window, &event);
            if response.repaint {
                state.window.request_redraw();
            }
            egui_consumed = response.consumed;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key:
                            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if !egui_consumed {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size.width, size.height);
                    state.window.request_redraw();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if egui_consumed {
                    return;
                }
                if button == MouseButton::Left {
                    self.is_dragging = state == ElementState::Pressed;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = Pt {
                    x: position.x,
                    y: position.y,
                };

                if egui_consumed {
                    self.cursor_pos = new_pos;
                    return;
                }

                // Only drag if button is held AND we are zoomed in
                if self.is_dragging && self.user_zoom > 1.0 {
                    let dx = new_pos.x - self.cursor_pos.x;
                    let dy = new_pos.y - self.cursor_pos.y;

                    self.user_pan.x += dx;
                    self.user_pan.y += dy;

                    // Constrain the pan so we don't drag out of the image
                    if let Some(state) = &self.state {
                        let size = state.window.inner_size();
                        let w = size.width as f64;
                        let h = size.height as f64;

                        // Calculate max allowed pan offset from center
                        // This ensures the screen viewport stays within the zoomed image bounds
                        let limit_x = (w * (self.user_zoom - 1.0) / 2.0).max(0.0);
                        let limit_y = (h * (self.user_zoom - 1.0) / 2.0).max(0.0);

                        self.user_pan.x = self.user_pan.x.clamp(-limit_x, limit_x);
                        self.user_pan.y = self.user_pan.y.clamp(-limit_y, limit_y);

                        state.window.request_redraw();
                    }
                }

                self.cursor_pos = new_pos;
            }

            // (Previous Zoom Logic)
            WindowEvent::MouseWheel { delta, .. } => {
                if egui_consumed {
                    return;
                }

                let zoom_factor = match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        if y > 0.0 {
                            1.1
                        } else {
                            0.9
                        }
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        if pos.y > 0.0 {
                            1.05
                        } else {
                            0.95
                        }
                    }
                };

                let mut new_zoom = self.user_zoom * zoom_factor;

                if let Some(state) = &self.state {
                    let size = state.window.inner_size();
                    let mx = self.cursor_pos.x - (size.width as f64 / 2.0);
                    let my = self.cursor_pos.y - (size.height as f64 / 2.0);

                    let p_x = mx - (mx - self.user_pan.x) * (new_zoom / self.user_zoom);
                    let p_y = my - (my - self.user_pan.y) * (new_zoom / self.user_zoom);

                    // Reset to center if zoomed out
                    if new_zoom <= 1.001 {
                        new_zoom = 1.0;
                        self.user_pan = Pt { x: 0.0, y: 0.0 };
                    } else {
                        new_zoom = new_zoom.clamp(1.0, 1000.0);
                        self.user_pan = Pt { x: p_x, y: p_y };
                    }

                    // Re-clamp in case zoom changed constraints (optional but safe)
                    let limit_x = (size.width as f64 * (new_zoom - 1.0) / 2.0).max(0.0);
                    let limit_y = (size.height as f64 * (new_zoom - 1.0) / 2.0).max(0.0);
                    self.user_pan.x = self.user_pan.x.clamp(-limit_x, limit_x);
                    self.user_pan.y = self.user_pan.y.clamp(-limit_y, limit_y);

                    self.user_zoom = new_zoom;
                    state.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                self.process_pending_text();
                // Prepare quit flag reference to capture in closure
                let quit_flag = &mut self.quit_requested;
                let mut replot_needed = false;
                if let Some(state) = &mut self.state {
                    replot_needed = state
                        .render(
                            &self.data,
                            self.viewbox.clone(),
                            self.user_zoom,
                            self.user_pan,
                            |ctx| {
                                let mut replot = false;
                                egui::Window::new("Plotter Menu")
                                    .collapsible(true)
                                    .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
                                    .show(ctx, |ui| {
                                        ui.heading("Controls");
                                        ui.label(format!("Zoom: {:.2}", self.user_zoom));

                                        if ui.button("Reset View").clicked() {
                                            self.user_zoom = 1.0;
                                            self.user_pan = Pt { x: 0.0, y: 0.0 };
                                        }

                                        if !self.pages.is_empty() {
                                            ui.separator();
                                            let selected_text = self
                                                .pages
                                                .get(self.active_page)
                                                .cloned()
                                                .unwrap_or_default();
                                            egui::ComboBox::from_label("Page")
                                                .selected_text(selected_text.0)
                                                .show_ui(ui, |ui| {
                                                    for (i, page_name) in
                                                        self.pages.iter().enumerate()
                                                    {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self.active_page,
                                                                i,
                                                                page_name.0.clone(),
                                                            )
                                                            .changed()
                                                        {
                                                            replot = true;
                                                            // Reset view so the new page fits nicely
                                                            self.user_zoom = 1.0;
                                                            self.user_pan = Pt { x: 0.0, y: 0.0 };
                                                        }
                                                    }
                                                });
                                        }

                                        if let Some(cb) = &mut self.ui_callback {
                                            ui.separator();
                                            if cb(ui) {
                                                replot = true;
                                            }
                                        }

                                        ui.separator();
                                        if ui.button("Quit").clicked() {
                                            *quit_flag = true;
                                        }
                                    });
                                replot
                            },
                        )
                        .unwrap();
                }

                if replot_needed {
                    self.clear();
                    if let Some(mut cb) = self.replot_callback.take() {
                        cb(self);
                        self.replot_callback = Some(cb);
                    }
                }

                // Check quit flag after render
                if self.quit_requested {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.wait_for_idle();
        }
        self.state.take();
    }
}
