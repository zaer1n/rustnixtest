use clap::Parser;
use pollster::FutureExt;
use std::error::Error;
use std::iter;
use std::path::PathBuf;
use twmap::{LayerKind, LoadMultiple};
use vek::{Extent2, Vec2};

use twgpu::map::{GpuMapData, GpuMapStatic};
use twgpu::textures::Samplers;
use twgpu::{device_descriptor, Camera, GpuCamera, TwRenderPass};
use twgpu_tools::{parse_tuple, DownloadTexture};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const LABEL: Option<&str> = Some("Map Photography");

pub fn make_screenshot() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut map: PathBuf = PathBuf::from("./maps/Mixi4Rouz.map");
    let mut zoom: Vec<f32> = vec![1.25];
    let mut resolution: Vec<(u32, u32)> = vec![(1920, 1080)];
    let mut position: Vec<(f32, f32)> = vec![(50, 50)];


    println!("Loading map");
    let mut map = twmap::TwMap::parse_path(map)?;
    let game_layer = map.find_physics_layer::<twmap::GameLayer>().unwrap();
    let map_size: Extent2<f32> = game_layer.tiles.shape().az();

    map.embed_images_auto()?;
    map.images.load()?;
    map.groups
        .load_conditionally(|layer| layer.kind() == LayerKind::Tiles)?;

    println!("Connecting to GPU backend");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .block_on()
        .expect("No suitable adapter found");
    let (device, queue) = adapter
        .request_device(&device_descriptor(&adapter), None)
        .block_on()?;

    println!("Uploading data to GPU");
    let mut camera = Camera::new(1.);
    let gpu_camera = GpuCamera::upload(&camera, &device);
    let samplers = Samplers::new(&device);
    let map_static = GpuMapStatic::new(FORMAT, &device);
    let map_data = GpuMapData::upload(&map, &device, &queue);
    let map_render = map_static.prepare_render(&map, &map_data, &gpu_camera, &samplers, &device);

    println!("Rendering pictures as PNG");
    let download_texture = DownloadTexture::new(resolution.0, resolution.1, FORMAT, &device);
    camera.switch_aspect_ratio(resolution.0 as f32 / resolution.1 as f32);
    camera.zoom = [zoom, zoom].into();
    camera.position = position.into();
    gpu_camera.update(&camera, &queue);
    map_data.update(&map, &camera, resolution.into(), 0, 0, &queue);
    let view = download_texture.texture_view();
    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: LABEL });
    {
        let render_pass =
            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: LABEL,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        let mut tw_render_pass =
            TwRenderPass::new(render_pass, resolution.into(), &camera);
        map_render.render_background(&mut tw_render_pass);
        map_render.render_foreground(&mut tw_render_pass);
    }
    queue.submit(iter::once(command_encoder.finish()));
    let image = download_texture.download_rgba(&device, &queue);
    image.save(PathBuf::from("./map.png"))?;
    println!("Saved.");
    Ok(())
}
