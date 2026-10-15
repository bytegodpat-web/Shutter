//! Backend GPU inicial de Shutter.
//!
//! Esta versión dibuja formas rellenas mediante mallas de triángulos en una
//! textura RGBA8 fuera de pantalla y permite leer el resultado como RGBA
//! premultiplicado. Aún no implementa antialiasing, grupos, máscaras ni modos
//! de mezcla distintos de Normal.

use std::{borrow::Cow, sync::mpsc};

use shutter_core::{Background, BlendMode, ShapeKind};
use shutter_render::{Primitive, Scene};
use wgpu::util::DeviceExt;

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Errores recuperables del backend inicial.
#[derive(Debug)]
pub enum GpuError {
    NoAdapter(String),
    RequestDevice(String),
    InvalidSize,
    UnsupportedBlend(BlendMode),
    Readback(String),
    DevicePoll(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter(e) => write!(f, "no se encontró un adaptador wgpu: {e}"),
            Self::RequestDevice(e) => write!(f, "no se pudo crear el dispositivo wgpu: {e}"),
            Self::InvalidSize => write!(f, "el tamaño de la composición debe ser mayor que cero"),
            Self::UnsupportedBlend(mode) => {
                write!(f, "el backend wgpu inicial aún no admite el modo {mode:?}")
            }
            Self::Readback(e) => write!(f, "falló la lectura de la textura GPU: {e}"),
            Self::DevicePoll(e) => write!(f, "falló la espera del dispositivo GPU: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// Imagen GPU leída de vuelta a CPU en RGBA8 premultiplicado.
#[derive(Clone, Debug)]
pub struct GpuFrame {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
}

impl GpuFrame {
    pub fn data(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "píxel fuera de la imagen");
        let offset = ((y * self.width + x) * 4) as usize;
        self.pixels[offset..offset + 4].try_into().expect("cuatro canales RGBA")
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

/// Renderizador de composición fuera de pantalla. Se crea una vez y se reutiliza.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
}

impl GpuRenderer {
    /// Crea el adaptador, dispositivo y pipeline con las features portables por defecto.
    pub fn new() -> Result<Self, GpuError> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|e| GpuError::NoAdapter(e.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| GpuError::RequestDevice(e.to_string()))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shutter-render-wgpu-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shutter-render-wgpu-pipeline-layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shutter-render-wgpu-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self { device, queue, pipeline })
    }

    /// Renderiza una Scene en GPU y devuelve RGBA8 premultiplicado.
    ///
    /// Esta versión acepta solo BlendMode::Normal; los demás modos producen
    /// un error explícito para no cambiar silenciosamente su semántica.
    pub fn render(&self, scene: &Scene) -> Result<GpuFrame, GpuError> {
        if scene.width == 0 || scene.height == 0 {
            return Err(GpuError::InvalidSize);
        }

        let mut vertices = Vec::new();
        let mut draws: Vec<std::ops::Range<u32>> = Vec::new();

        for item in &scene.items {
            if item.blend != BlendMode::Normal {
                return Err(GpuError::UnsupportedBlend(item.blend));
            }
            let Some(color) = item.fill else { continue };
            let Primitive::Shape { kind, width, height } = &item.primitive;
            let points = shape_points(kind, *width, *height);
            if points.len() < 3 {
                continue;
            }

            let alpha = (color[3] * item.opacity).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }
            // El framebuffer y el rasterizador CPU usan alpha premultiplicado.
            let rgba = [
                color[0].clamp(0.0, 1.0) * alpha,
                color[1].clamp(0.0, 1.0) * alpha,
                color[2].clamp(0.0, 1.0) * alpha,
                alpha,
            ];

            let first = vertices.len() as u32;
            let center = to_clip(item.transform.apply(0.0, 0.0), scene.width, scene.height);
            for (index, point) in points.iter().enumerate() {
                let a = to_clip(item.transform.apply(point.0, point.1), scene.width, scene.height);
                let next = (index + 1) % points.len();
                let next_point = points[next];
                let b = to_clip(item.transform.apply(next_point.0, next_point.1), scene.width, scene.height);
                vertices.push(Vertex { position: center, color: rgba });
                vertices.push(Vertex { position: a, color: rgba });
                vertices.push(Vertex { position: b, color: rgba });
            }
            draws.push(first..vertices.len() as u32);
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shutter-render-wgpu-output"),
            size: wgpu::Extent3d { width: scene.width, height: scene.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let vertex_buffer = (!vertices.is_empty()).then(|| {
            self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shutter-render-wgpu-vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });

        let unpadded_bytes_per_row = scene.width.checked_mul(4).ok_or(GpuError::InvalidSize)?;
        let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(scene.height))
            .ok_or(GpuError::InvalidSize)?;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shutter-render-wgpu-readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("shutter-render-wgpu-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shutter-render-wgpu-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(background_color(scene.background)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(buffer) = &vertex_buffer {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                for range in &draws {
                    pass.draw(range.clone(), 0..1);
                }
            }
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(scene.height),
                },
            },
            wgpu::Extent3d { width: scene.width, height: scene.height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| GpuError::DevicePoll(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| GpuError::Readback(e.to_string()))?
            .map_err(|e| GpuError::Readback(e.to_string()))?;

        let mapped = slice.get_mapped_range().map_err(|e| GpuError::Readback(e.to_string()))?;
        let mut pixels = vec![0; (u64::from(scene.width) * u64::from(scene.height) * 4) as usize];
        for row in 0..scene.height as usize {
            let src_start = row * padded_bytes_per_row as usize;
            let dst_start = row * scene.width as usize * 4;
            let row_bytes = scene.width as usize * 4;
            pixels[dst_start..dst_start + row_bytes]
                .copy_from_slice(&mapped[src_start..src_start + row_bytes]);
        }
        drop(mapped);
        readback.unmap();

        Ok(GpuFrame { width: scene.width, height: scene.height, pixels })
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn background_color(background: Background) -> wgpu::Color {
    match background {
        Background::Black => wgpu::Color::BLACK,
        Background::LightGray => wgpu::Color { r: 217.0 / 255.0, g: 217.0 / 255.0, b: 217.0 / 255.0, a: 1.0 },
        Background::White => wgpu::Color::WHITE,
        Background::Green => wgpu::Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
        Background::Blue => wgpu::Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },
        Background::Transparent => wgpu::Color::TRANSPARENT,
    }
}

fn to_clip(point: (f32, f32), width: u32, height: u32) -> [f32; 2] {
    [point.0 / width as f32 * 2.0 - 1.0, 1.0 - point.1 / height as f32 * 2.0]
}

/// Perímetro centrado en el origen. Se triangula como un abanico durante el render.
fn shape_points(kind: &ShapeKind, width: f32, height: f32) -> Vec<(f32, f32)> {
    use std::f32::consts::TAU;
    let (w, h) = (width.abs(), height.abs());
    if w == 0.0 || h == 0.0 {
        return Vec::new();
    }

    match kind {
        ShapeKind::Rectangle => {
            vec![(-w / 2.0, -h / 2.0), (w / 2.0, -h / 2.0), (w / 2.0, h / 2.0), (-w / 2.0, h / 2.0)]
        }
        ShapeKind::Ellipse => (0..64)
            .map(|i| {
                let angle = i as f32 / 64.0 * TAU - std::f32::consts::FRAC_PI_2;
                (w / 2.0 * angle.cos(), h / 2.0 * angle.sin())
            })
            .collect(),
        ShapeKind::Polygon { sides } => {
            let count = (*sides).max(3);
            (0..count)
                .map(|i| {
                    let angle = i as f32 / count as f32 * TAU - std::f32::consts::FRAC_PI_2;
                    (w / 2.0 * angle.cos(), h / 2.0 * angle.sin())
                })
                .collect()
        }
        ShapeKind::Star { points, inner_ratio } => {
            let count = (*points).max(2);
            let inner = inner_ratio.clamp(0.05, 1.0);
            (0..count * 2)
                .map(|i| {
                    let radius = if i % 2 == 0 { 1.0 } else { inner };
                    let angle = i as f32 / (count * 2) as f32 * TAU - std::f32::consts::FRAC_PI_2;
                    (radius * w / 2.0 * angle.cos(), radius * h / 2.0 * angle.sin())
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shutter_render::Affine;

    #[test]
    fn row_alignment_handles_unaligned_widths() {
        assert_eq!(align_to(4, 256), 256);
        assert_eq!(align_to(256, 256), 256);
        assert_eq!(align_to(260, 256), 512);
    }

    #[test]
    fn rectangle_geometry_is_centered() {
        let points = shape_points(&ShapeKind::Rectangle, 20.0, 10.0);
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], (-10.0, -5.0));
        assert_eq!(points[2], (10.0, 5.0));
    }

    #[test]
    fn gpu_matches_cpu_at_solid_rectangle_center_when_adapter_exists() {
        let renderer = match GpuRenderer::new() {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("prueba GPU omitida: {error}");
                return;
            }
        };
        let scene = Scene {
            width: 32,
            height: 32,
            background: Background::Transparent,
            items: vec![shutter_render::Item {
                layer: 1,
                primitive: Primitive::Shape { kind: ShapeKind::Rectangle, width: 16.0, height: 16.0 },
                transform: Affine::translate(16.0, 16.0),
                opacity: 1.0,
                fill: Some([1.0, 0.0, 0.0, 1.0]),
                blend: BlendMode::Normal,
            }],
        };
        let gpu = renderer.render(&scene).expect("render GPU");
        let cpu = shutter_render::render(&scene).expect("render CPU");
        assert_eq!(gpu.pixel(16, 16), cpu.pixel(16, 16));
        assert_eq!(gpu.pixel(0, 0), [0, 0, 0, 0]);
        assert_eq!(gpu.width, cpu.width);
        assert_eq!(gpu.height, cpu.height);
    }
}
