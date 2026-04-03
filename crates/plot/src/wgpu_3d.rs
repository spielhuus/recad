use super::{Paint, Plotter};
use font::OSIFONT;
use types::{
    constants::FONT_SCALING, gr::{Effects, Pos, Pt, Pts, Rect}
};
use glam::{DVec2, Mat4, Vec3};
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PrimitiveInstance {
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    z_index: f32,
    _pad1: f32,
    color: [f32; 4],
    width: f32,
    radius: f32,
    type_id: u32,
    angle: f32,
}

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    pub uv_rect: [f64; 4],
    pub size: [f32; 2],
    pub bearing: [f32; 2],
    pub advance: f32,
}

pub struct Wgpu3dPlotter {
    viewbox: Option<Rect>,
    data: Vec<PrimitiveInstance>,
    pending_text: Vec<(String, Pos, Effects)>,
    pending_indices: Vec<usize>,
    current_pos: Pt,

    proxy: Option<winit::event_loop::EventLoopProxy<super::wgpu::UserEvent>>,
    state: Option<State3D>,
    user_zoom: f64,
    user_pan: Pt,
    cursor_pos: Pt,
    is_left_dragging: bool,
    is_right_dragging: bool,
    camera_pitch: f64,
    camera_yaw: f64,
    quit_requested: bool,

    #[allow(clippy::type_complexity)]
    ui_callback: Option<Box<dyn FnMut(&mut egui::Ui) -> bool>>,
    #[allow(clippy::type_complexity)]
    replot_callback: Option<Box<dyn FnMut(&mut Self)>>,
}

impl Wgpu3dPlotter {
    pub fn new(event_loop: &EventLoop<super::wgpu::UserEvent>) -> Self {
        Wgpu3dPlotter {
            state: None,
            proxy: Some(event_loop.create_proxy()),
            viewbox: None,
            data: Vec::new(),
            pending_text: Vec::new(),
            pending_indices: Vec::new(),
            current_pos: Pt { x: 0.0, y: 0.0 },
            user_zoom: 1.0,
            user_pan: Pt { x: 0.0, y: 0.0 },
            cursor_pos: Pt { x: 0.0, y: 0.0 },
            is_left_dragging: false,
            is_right_dragging: false,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            quit_requested: false,
            ui_callback: None,
            replot_callback: None,
        }
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

    fn clean_points(&self, indices: &[usize]) -> Vec<usize> {
        let mut current = indices.to_vec();
        let mut changed = true;
        while changed {
            changed = false;
            if current.len() < 3 {
                break;
            }
            let mut next_pass = Vec::new();
            let len = current.len();
            for i in 0..len {
                let prev = current[(i + len - 1) % len];
                let curr = current[i];
                let next = current[(i + 1) % len];
                let p_prev = DVec2::new(self.data[prev].p1[0] as f64, self.data[prev].p1[1] as f64);
                let p_curr = DVec2::new(self.data[curr].p1[0] as f64, self.data[curr].p1[1] as f64);
                let p_next = DVec2::new(self.data[next].p1[0] as f64, self.data[next].p1[1] as f64);
                if (p_curr - p_prev).length() < 0.001 || (p_prev - p_next).length() < 0.001 {
                    changed = true;
                    continue;
                }
                let v1 = p_curr - p_prev;
                let v2 = p_next - p_curr;
                if (self.cross(v1, v2).abs() < 0.001) && (v1.dot(v2) > 0.0) {
                    changed = true;
                    continue;
                }
                next_pass.push(curr);
            }
            current = next_pass;
        }
        current
    }

    fn triangulate(&mut self, index_list: &[usize]) -> Vec<[DVec2; 3]> {
        if self.data.len() < 3 {
            return vec![];
        }
        let mut index_list = self.clean_points(index_list);
        let mut sum = 0.0;
        for i in 0..index_list.len() {
            let p1 = self.data[index_list[i]].p1;
            let p2 = self.data[index_list[(i + 1) % index_list.len()]].p1;
            sum += (p2[0] - p1[0]) * (p2[1] + p1[1]);
        }
        let should_be_positive = sum > 0.0;
        let mut total_triangles = vec![];
        let mut safety_count = 0;
        let max_iterations = self.data.len() * 3;

        while index_list.len() > 3 {
            if safety_count > max_iterations {
                break;
            }
            let mut ear_found = false;
            for i in 0..index_list.len() {
                let a = self.get_item(i as isize, &index_list);
                let b = self.get_item(i as isize - 1, &index_list);
                let c = self.get_item(i as isize + 1, &index_list);
                let va = DVec2::new(self.data[a].p1[0] as f64, self.data[a].p1[1] as f64);
                let vb = DVec2::new(self.data[b].p1[0] as f64, self.data[b].p1[1] as f64);
                let vc = DVec2::new(self.data[c].p1[0] as f64, self.data[c].p1[1] as f64);
                let cp = self.cross(vb - va, vc - va);
                if !(if should_be_positive {
                    cp > 0.0
                } else {
                    cp < 0.0
                }) {
                    continue;
                }

                let mut is_ear = true;
                for other_idx in &index_list {
                    if *other_idx == a || *other_idx == b || *other_idx == c {
                        continue;
                    }
                    if self.in_triangle(
                        DVec2::new(
                            self.data[*other_idx].p1[0] as f64,
                            self.data[*other_idx].p1[1] as f64,
                        ),
                        vb,
                        va,
                        vc,
                    ) {
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
}

impl Plotter for Wgpu3dPlotter {
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
    fn scale(&mut self, _: f64) {}

    fn move_to(&mut self, pt: Pt) {
        self.current_pos = pt;
    }
    fn line_to(&mut self, pt: Pt) {
        self.data.push(PrimitiveInstance {
            p1: [self.current_pos.x as f32, self.current_pos.y as f32],
            p2: [pt.x as f32, pt.y as f32],
            p3: [0.0, 0.0],
            z_index: 0.0,
            _pad1: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
            width: 1.0,
            radius: 0.0,
            type_id: 0,
            angle: 0.0,
        });
        self.pending_indices.push(self.data.len() - 1);
        self.current_pos = pt;
    }

    fn close(&mut self) {}

    fn stroke(&mut self, stroke: Paint) {
        if let Some(fill_color) = stroke.fill {
            let mut index_list = self.pending_indices.clone();
            if let Some(&last_idx) = index_list.last() {
                if let Some(&first_idx) = index_list.first() {
                    let end = DVec2::new(
                        self.data[last_idx].p2[0] as f64,
                        self.data[last_idx].p2[1] as f64,
                    );
                    let start = DVec2::new(
                        self.data[first_idx].p1[0] as f64,
                        self.data[first_idx].p1[1] as f64,
                    );
                    if (end - start).length() > 0.001 {
                        self.data.push(PrimitiveInstance {
                            p1: self.data[last_idx].p2,
                            p2: self.data[last_idx].p2,
                            p3: [0.0, 0.0],
                            z_index: stroke.z_index,
                            _pad1: 0.0,
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
            for tri in self.triangulate(&index_list) {
                self.data.push(PrimitiveInstance {
                    p1: [tri[0].x as f32, tri[0].y as f32],
                    p2: [tri[1].x as f32, tri[1].y as f32],
                    p3: [tri[2].x as f32, tri[2].y as f32],
                    z_index: stroke.z_index,
                    _pad1: 0.0,
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
                instance.z_index = stroke.z_index;
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
            self.move_to(start);
            self.line_to(mid);
            self.line_to(end);
            self.stroke(stroke);
            return;
        }

        let d = 2.0 * (p1.x * (p2.y - p3.y) + p2.x * (p3.y - p1.y) + p3.x * (p1.y - p2.y));
        let center = DVec2::new(
            ((p1.x.powi(2) + p1.y.powi(2)) * (p2.y - p3.y)
                + (p2.x.powi(2) + p2.y.powi(2)) * (p3.y - p1.y)
                + (p3.x.powi(2) + p3.y.powi(2)) * (p1.y - p2.y))
                / d,
            ((p1.x.powi(2) + p1.y.powi(2)) * (p3.x - p2.x)
                + (p2.x.powi(2) + p2.y.powi(2)) * (p1.x - p3.x)
                + (p3.x.powi(2) + p3.y.powi(2)) * (p2.x - p1.x))
                / d,
        );
        let radius = (p1 - center).length();
        let angle_start = (p1.y - center.y).atan2(p1.x - center.x);
        let angle_mid = (p2.y - center.y).atan2(p2.x - center.x);
        let angle_end = (p3.y - center.y).atan2(p3.x - center.x);

        let normalize = |mut a: f64| -> f64 {
            while a <= -std::f64::consts::PI {
                a += 2.0 * std::f64::consts::PI;
            }
            while a > std::f64::consts::PI {
                a -= 2.0 * std::f64::consts::PI;
            }
            a
        };
        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [
                angle_start as f32,
                normalize(angle_mid - angle_start) as f32,
            ],
            p3: [0.0, 0.0],
            z_index: stroke.z_index,
            _pad1: 0.0,
            color: stroke.color.into(),
            width: stroke.width as f32,
            radius: radius as f32,
            type_id: 5,
            angle: 0.0,
        });
        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [angle_mid as f32, normalize(angle_end - angle_mid) as f32],
            p3: [0.0, 0.0],
            z_index: stroke.z_index,
            _pad1: 0.0,
            color: stroke.color.into(),
            width: stroke.width as f32,
            radius: radius as f32,
            type_id: 5,
            angle: 0.0,
        });
        self.current_pos = end;
    }

    fn circle(&mut self, center: Pt, radius: f64, stroke: Paint) {
        let is_filled = stroke.fill.is_some();
        self.data.push(PrimitiveInstance {
            p1: [center.x as f32, center.y as f32],
            p2: [0.0, 0.0],
            p3: [0.0, 0.0],
            z_index: stroke.z_index,
            _pad1: 0.0,
            color: if is_filled {
                stroke.fill.unwrap().into()
            } else {
                stroke.color.into()
            },
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
        let state = match &self.state {
            Some(s) => s,
            None => {
                self.pending_text.push((text.to_string(), pos, effects));
                return;
            }
        };
        let font_scale = (effects.font.size.0 * FONT_SCALING) / 64.0;
        let angle_rad = pos.angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let rotate =
            |x: f64, y: f64| -> (f64, f64) { (x * cos_a - y * sin_a, x * sin_a + y * cos_a) };

        let mut cursor_x = pos.x;
        let mut cursor_y = pos.y;
        for c in text.chars() {
            if let Some(info) = state.font_lookup.get(&c) {
                let width = (info.size[0] * font_scale) as f64;
                let height = (info.size[1] * font_scale) as f64;
                let bearing_x = (info.bearing[0] * font_scale) as f64;
                let bearing_y = (info.bearing[1] * font_scale) as f64;
                let (rot_cx, rot_cy) =
                    rotate(bearing_x + (width / 2.0), bearing_y + (height / 2.0));

                if width > 0.0 {
                    self.data.push(PrimitiveInstance {
                        p1: [(cursor_x + rot_cx) as f32, (cursor_y + rot_cy) as f32],
                        p2: [width as f32, height as f32],
                        p3: [info.uv_rect[0] as f32, info.uv_rect[1] as f32],
                        z_index: effects.z_index,
                        _pad1: 0.0,
                        color: effects.font.color.unwrap_or_default().into(),
                        width: info.uv_rect[2] as f32,
                        radius: info.uv_rect[3] as f32,
                        type_id: 4,
                        angle: angle_rad as f32,
                    });
                }
                let (adv_x, adv_y) = rotate(info.advance as f64 * font_scale as f64, 0.0);
                cursor_x += adv_x;
                cursor_y += adv_y;
            }
        }
    }
}

pub struct State3D {
    window: Arc<Window>,
    device: ::wgpu::Device,
    queue: ::wgpu::Queue,
    surface: ::wgpu::Surface<'static>,
    config: ::wgpu::SurfaceConfiguration,
    depth_texture: ::wgpu::Texture,
    depth_view: ::wgpu::TextureView,
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

impl State3D {
    async fn new(window: Arc<Window>) -> Result<State3D, Box<dyn std::error::Error>> {
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
            .request_device(&::wgpu::DeviceDescriptor::default())
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

        let depth_texture = device.create_texture(&::wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: ::wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: ::wgpu::TextureDimension::D2,
            format: ::wgpu::TextureFormat::Depth32Float,
            usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&::wgpu::TextureViewDescriptor::default());

        let shader = device.create_shader_module(::wgpu::ShaderModuleDescriptor {
            label: Some("Shader 3D"),
            source: ::wgpu::ShaderSource::Wgsl(include_str!("shader_3d.wgsl").into()),
        });
        let quad_vertex_buffer = device.create_buffer_init(&::wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: ::wgpu::BufferUsages::VERTEX,
        });
        let initial_instances = 100;
        let instance_buffer = device.create_buffer(&::wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (initial_instances * std::mem::size_of::<PrimitiveInstance>()) as u64,
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
                label: Some("Uniform BGL"),
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
                label: Some("Tex BGL"),
            });
        let render_pipeline_layout =
            device.create_pipeline_layout(&::wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&::wgpu::RenderPipelineDescriptor {
            label: Some("3D Render Pipeline"),
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
                            ::wgpu::VertexAttribute {
                                offset: 24,
                                shader_location: 8,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 32,
                                shader_location: 9,
                                format: ::wgpu::VertexFormat::Float32x4,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 48,
                                shader_location: 10,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 52,
                                shader_location: 11,
                                format: ::wgpu::VertexFormat::Float32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 56,
                                shader_location: 12,
                                format: ::wgpu::VertexFormat::Uint32,
                            },
                            ::wgpu::VertexAttribute {
                                offset: 60,
                                shader_location: 13,
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
            depth_stencil: Some(::wgpu::DepthStencilState {
                format: ::wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: ::wgpu::CompareFunction::LessEqual,
                stencil: ::wgpu::StencilState::default(),
                bias: ::wgpu::DepthBiasState::default(),
            }),
            multisample: ::wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        #[cfg(not(target_arch = "wasm32"))]
        let egui_renderer = egui_utils::EguiRenderer::new(&device, config.format, &window);

        let font_bytes = OSIFONT.to_vec();
        let font = Font::from_bytes(font_bytes, FontSettings::default()).unwrap();
        let mut font_lookup = HashMap::new();
        let mut atlas_data = vec![0u8; 512 * 512];
        let atlas_w = 512;
        let atlas_h = 512;
        let mut cx = 0;
        let mut cy = 0;
        let pad = 2;

        for ch in
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,!?:;+-=()[]{} \"'/_%@#"
                .chars()
        {
            let (metrics, bitmap) = font.rasterize(ch, 64.0);
            if metrics.width > 0 {
                let w = metrics.width as u32;
                let h = metrics.height as u32;
                if cx + w + pad > atlas_w {
                    cx = 0;
                    cy += 64 + pad;
                }
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
            depth_texture,
            depth_view,
            render_pipeline,
            font_lookup,
            uniform_buffer,
            uniform_bind_group,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: initial_instances,
            diffuse_bind_group,
            #[cfg(not(target_arch = "wasm32"))]
            egui_renderer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = self.device.create_texture(&::wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: ::wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: ::wgpu::TextureDimension::D2,
                format: ::wgpu::TextureFormat::Depth32Float,
                usage: ::wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth_view = self
                .depth_texture
                .create_view(&::wgpu::TextureViewDescriptor::default());
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

    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        primitives: &[PrimitiveInstance],
        viewbox: Option<Rect>,
        user_zoom: f64,
        user_pan: Pt,
        camera_pitch: f64,
        camera_yaw: f64,
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
                    label: Some("Instance Buffer"),
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
                (
                    vb.start.x as f32,
                    vb.start.y as f32,
                    vb.end.x as f32,
                    vb.end.y as f32,
                )
            } else {
                (0.0, 0.0, width, height)
            };
            let data_center_x = min_x + data_w / 2.0;
            let data_center_y = min_y + data_h / 2.0;

            let fov = 45.0_f32.to_radians();
            let aspect = width / height;
            let proj = Mat4::perspective_rh(fov, aspect, 0.1, 10000.0);

            let target = Vec3::new(
                data_center_x - user_pan.x as f32,
                data_center_y - user_pan.y as f32,
                0.0,
            );
            let max_dim = data_w.max(data_h);
            let base_distance = (max_dim / 2.0) / (fov / 2.0).tan();
            let distance = base_distance / user_zoom.max(0.1) as f32;

            let pitch = camera_pitch as f32;
            let yaw = camera_yaw as f32;
            let eye = target
                + Vec3::new(
                    yaw.sin() * pitch.cos(),
                    pitch.sin(),
                    yaw.cos() * pitch.cos(),
                ) * distance;
            let view_mat = Mat4::look_at_rh(eye, target, Vec3::new(0.0, -1.0, 0.0));

            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[CameraUniform {
                    view_proj: (proj * view_mat).to_cols_array_2d(),
                    screen_size: [width, height],
                    _padding: [0.0; 2],
                }]),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&::wgpu::RenderPassDescriptor {
                    label: Some("3D Render Pass"),
                    color_attachments: &[Some(::wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: ::wgpu::Operations {
                            load: ::wgpu::LoadOp::Clear(::wgpu::Color::BLACK),
                            store: ::wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(::wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(::wgpu::Operations {
                            load: ::wgpu::LoadOp::Clear(1.0),
                            store: ::wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
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
                let sd = egui_wgpu::ScreenDescriptor {
                    size_in_pixels: [width as u32, height as u32],
                    pixels_per_point: self.window.scale_factor() as f32,
                };
                self.egui_renderer.end_frame_and_draw(
                    &self.device,
                    &self.queue,
                    &mut encoder,
                    &self.window,
                    &view,
                    sd,
                );
            }
            self.queue.submit(std::iter::once(encoder.finish()));
        }
        output.present();
        Ok(replot)
    }
}

impl ApplicationHandler<super::wgpu::UserEvent> for Wgpu3dPlotter {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State3D::new(window)).unwrap());
        }
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: super::wgpu::UserEvent) {}

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
                    self.is_left_dragging = state == ElementState::Pressed;
                }
                if button == MouseButton::Right {
                    self.is_right_dragging = state == ElementState::Pressed;
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

                let dx = new_pos.x - self.cursor_pos.x;
                let dy = new_pos.y - self.cursor_pos.y;
                if self.is_left_dragging {
                    self.camera_yaw += dx * 0.01;
                    self.camera_pitch += dy * 0.01;
                    self.camera_pitch = self.camera_pitch.clamp(
                        -std::f64::consts::PI / 2.0 + 0.1,
                        std::f64::consts::PI / 2.0 - 0.1,
                    );
                    if let Some(state) = &self.state {
                        state.window.request_redraw();
                    }
                } else if self.is_right_dragging {
                    let pan_speed = 1.0 / self.user_zoom.max(0.1);
                    self.user_pan.x += dx * pan_speed;
                    self.user_pan.y += dy * pan_speed;
                    if let Some(state) = &self.state {
                        state.window.request_redraw();
                    }
                }
                self.cursor_pos = new_pos;
            }
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
                self.user_zoom = (self.user_zoom * zoom_factor).clamp(0.01, 1000.0);
                if let Some(state) = &self.state {
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.state.is_some() && !self.pending_text.is_empty() {
                    let pending = std::mem::take(&mut self.pending_text);
                    for (text, pos, effects) in pending {
                        self.text(&text, pos, effects);
                    }
                }

                let quit_flag = &mut self.quit_requested;
                let mut replot_needed = false;
                if let Some(state) = &mut self.state {
                    replot_needed = state
                        .render(
                            &self.data,
                            self.viewbox.clone(),
                            self.user_zoom,
                            self.user_pan,
                            self.camera_pitch,
                            self.camera_yaw,
                            |ctx| {
                                let mut replot = false;
                                egui::Window::new("3D Plotter")
                                    .collapsible(true)
                                    .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
                                    .show(ctx, |ui| {
                                        ui.heading("Orbit Controls");
                                        ui.label(format!("Zoom: {:.2}", self.user_zoom));
                                        if ui.button("Reset View").clicked() {
                                            self.user_zoom = 1.0;
                                            self.user_pan = Pt { x: 0.0, y: 0.0 };
                                            self.camera_pitch = 0.0;
                                            self.camera_yaw = 0.0;
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
