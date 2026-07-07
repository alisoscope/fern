#![feature(iter_array_chunks)]

use bytemuck::cast_slice;
use fern::{graphics::PresentationSubSystemImpl, *};

use prelude::*;

pub fn main() -> anyhow::Result<()> {
    let _logger = logging::GlobalLoggerContext::init(
        logging::LoggerConfig {
            log_out_mode: logging::LogOutMode::PrintWithAnsiCodes,
            trim_newlines: true,
        },
        true,
    );

    let mut app = app::App::new("hello world!")
        .inspect_err(|e| log_release!(LogType::Error, "App init error: {}.", e))?;

    use math::mesh::*;

    fn load_mesh(file_name: &str) -> Mesh<TexturedVertex> {
        let (suzanne, _) = tobj::load_obj(file_name, &tobj::GPU_LOAD_OPTIONS).unwrap();


        let suzanne = suzanne.into_iter().next().unwrap().mesh;
        let suzanne = Mesh::<TexturedVertex> {
            vertices: suzanne.positions.into_iter()
                .array_chunks::<3>()
                .map(|v| glam::Vec3::from_array(v))
                .zip(suzanne.normals.into_iter()
                    .array_chunks::<3>()
                    .map(|v| glam::Vec3::from_array(v))
                )
                .zip(suzanne.texcoords.into_iter()
                    .array_chunks::<2>()
                    .map(|v| glam::Vec2::from_array(v))
                )
                .map(|((position, normal), tex_coord)| TexturedVertex {
                    position,
                    normal,
                    tex_coord,
                })
                .collect(),
            indices: suzanne.indices,
        };

        suzanne
    }

    let suzanne = load_mesh("meshes/suzanne.obj");
    let cube = load_mesh("meshes/cube.obj");
    let sphere = load_mesh("meshes/cone.obj");

    let buffer_len = (
        (suzanne.vertices.len() + cube.vertices.len() + sphere.vertices.len()) * size_of::<TexturedVertex>()
    ) as u64;

    let suzanne_bytes = (suzanne.vertices.len() * size_of::<TexturedVertex>()) as u64;
    let suzanne_base_vertex = 0;
    let suzanne_base_vertex_bytes = 0;
    let cube_bytes = (cube.vertices.len() * size_of::<TexturedVertex>()) as u64;
    let cube_base_vertex = suzanne.vertices.len() as u64;
    let cube_base_vertex_bytes = cube_base_vertex * size_of::<TexturedVertex>() as u64;
    let sphere_bytes = (sphere.vertices.len() * size_of::<TexturedVertex>()) as u64;
    let sphere_base_vertex = cube_base_vertex + cube.vertices.len() as u64;
    let sphere_base_vertex_bytes = sphere_base_vertex * size_of::<TexturedVertex>() as u64;


    let vertex_buffer = app.graphics.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer"),
        size: suzanne_bytes + cube_bytes + sphere_bytes,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });

    vertex_buffer.get_mapped_range_mut(suzanne_base_vertex_bytes..suzanne_base_vertex_bytes + suzanne_bytes)
        .copy_from_slice(bytemuck::cast_slice(&suzanne.vertices));
    vertex_buffer.get_mapped_range_mut(cube_base_vertex_bytes..cube_base_vertex_bytes + cube_bytes)
        .copy_from_slice(bytemuck::cast_slice(&cube.vertices));
    vertex_buffer.get_mapped_range_mut(sphere_base_vertex_bytes..sphere_base_vertex_bytes + sphere_bytes)
        .copy_from_slice(bytemuck::cast_slice(&sphere.vertices));

    let buffer_len = ((suzanne.indices.len() + cube.indices.len() + sphere.indices.len()) * size_of::<u32>()) as u64;

    let index_buffer = app.graphics.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("index buffer"),
        size: buffer_len,
        usage: wgpu::BufferUsages::INDEX,
        mapped_at_creation: true,
    });

    let suzanne_indicies_bytes = suzanne.indices.len() as u64 * 4;
    let cube_indicies_bytes = cube.indices.len() as u64 * 4;
    let sphere_indicies_bytes = sphere.indices.len() as u64 * 4;

    index_buffer.get_mapped_range_mut(0..suzanne_indicies_bytes)
        .copy_from_slice(bytemuck::cast_slice(&suzanne.indices));
    index_buffer.get_mapped_range_mut(suzanne_indicies_bytes..suzanne_indicies_bytes + cube_indicies_bytes)
        .copy_from_slice(bytemuck::cast_slice(&cube.indices));
    index_buffer.get_mapped_range_mut(suzanne_indicies_bytes + cube_indicies_bytes..suzanne_indicies_bytes + cube_indicies_bytes + sphere_indicies_bytes)
        .copy_from_slice(bytemuck::cast_slice(&sphere.indices));

    let suzanne_range = 0..suzanne.indices.len() as u32;
    let cube_range = suzanne_range.end..suzanne_range.end + cube.indices.len() as u32;
    let sphere_range = cube_range.end..cube_range.end + sphere.indices.len() as u32;

    vertex_buffer.unmap();
    index_buffer.unmap();

    let graphics= &app.graphics;

    #[repr(C)]
    struct Globals {
        view_proj: glam::Mat4,
        x: f32,
    }

    let globals_buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("view_proj buffer"),
        size: size_of::<Globals>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let model_buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("model buffer"),
        size: (size_of::<glam::Mat4>() as u64) * 3,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let textured_bind_group_layout = graphics.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("view_proj binding layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let textured_bind_group = graphics.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("view_proj bind group"),
        layout: &textured_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: model_buffer.as_entire_binding(),
            },
        ],
    });

    use std::fs;
    let shader_source = fs::read_to_string("shaders/textured_mesh.wgsl")?;

    let shader_module = graphics.shader_module(&shader_source);

    let pipeline_layout = graphics.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            Some(&textured_bind_group_layout),
        ],
        immediate_size: 0,
    });

    use std::mem::offset_of;
    use wgpu::VertexFormat;

    let pipeline = graphics.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<TexturedVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: offset_of!(TexturedVertex, position) as u64,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: offset_of!(TexturedVertex, normal) as u64,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: offset_of!(TexturedVertex, tex_coord) as u64,
                            shader_location: 2,
                        },
                    ],
                },
            ],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: app.presentation.surface_format(),
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let mut camera_position = glam::vec3(0f32, 10f32, 0f32);
    //let mut camera_rotation = glam::camera::rh::view::look_to_quat(glam::Vec3::Y, glam::Vec3::Z);

    let mut suzanne_position = glam::vec3(0f32, 0f32, 0f32);
    let mut cube_position = glam::vec3(5f32, 0f32, 5f32);
    let mut sphere_position = glam::vec3(-5f32, 0f32, -2f32);

    let mut camera_focal = glam::Vec3::ZERO;
    let mut camera_distance = 10f32;
    let mut camera_yaw = 0f32;
    let mut camera_pitch = 0f32;

    let mut i = 0f32;

    use input::ActionBinding;
    use sdl3::keyboard::Scancode;

    let space_action = app.input.register_action("space", ActionBinding::Scancode(Scancode::Space));
    let other_action = app.input.register_action("other", Scancode::Space.into());

    app.main_loop(|app| {
        //camera_rotation *= glam::Quat::from_axis_angle(glam::Vec3::Z, 0.01f32);
        let projection = glam::camera::rh::proj::directx::perspective_infinite(f32::to_radians(80f32), app.aspect_ratio, 0.1f32);

        if app.input.action_down(space_action) {
            log_release!(LogType::Info, "Space down.");
        }
        if app.input.action_pressed(space_action) {
            log_release!(LogType::Info, "Space pressed.");
        }
        if app.input.action_released(other_action) {
            log_release!(LogType::Info, "Other space released.");
        }

        let dt = 1f32 / 60f32;

        let mouse_sens = 0.1f32;
        let mouse_move_sens = 1f32;
        let mouse_scroll_sens = 0.1f32;

        /*
        camera_distance += app.io_state.scroll * mouse_scroll_sens;
        camera_distance= f32::clamp(camera_distance, 1f32, 50f32);

        if !app.io_state.mouse_state.right() {
            camera_yaw += -app.io_state.relative_mouse_state.x() * mouse_sens * dt;
            camera_pitch += app.io_state.relative_mouse_state.y() * mouse_sens * dt;
            camera_pitch = f32::clamp(camera_pitch, -std::f32::consts::PI / 2f32, std::f32::consts::PI / 2f32);
        }
        */

        let camera_rotation = 
            glam::Quat::from_axis_angle(glam::Vec3::Z, camera_yaw) *
            glam::Quat::from_axis_angle(glam::Vec3::X, camera_pitch);
        
        /*
        if app.io_state.mouse_state.right() {
            let camera_move = glam::vec3(app.io_state.relative_mouse_state.x(), 0.0, app.io_state.relative_mouse_state.y()) * mouse_move_sens * dt;
            let camera_move = camera_rotation * camera_move;
            camera_focal+= camera_move;
        }

*/
        /*
         *
         * IOSystem::poll(&mut IOStateAccumulator);
         * 
         * 
         * UIScene::update(&IOStateAccumulator);
         *
         *
         *
         */
        
        let camera_eye = glam::Vec3::Y * camera_distance;
        let camera_eye = camera_rotation * camera_eye;
        let camera_eye = camera_focal + camera_eye;

        use sdl3::keyboard::Scancode;

        //let view = glam::Mat4::from_rotation_translation(camera_rotation, camera_position);
        //let view = view.inverse();
        let view = glam::camera::rh::view::look_at_mat4(camera_eye, camera_focal, glam::Vec3::Z);
        let view_proj = projection * view;

        let globals = Globals {
            view_proj,
            x: f32::sin(i),
        };

        let globals_bytes = unsafe {
            std::slice::from_raw_parts(&raw const globals as *const u8, size_of::<Globals>())
        };

        //suzanne_position.z += 0.01;

        let model_matrices = [suzanne_position, cube_position, sphere_position].into_iter()
            .map(glam::Mat4::from_translation)
            .collect::<Vec<glam::Mat4>>();

        app.graphics.queue().write_buffer(&globals_buffer, 0, globals_bytes);
        app.graphics.queue().write_buffer(&model_buffer, 0, bytemuck::cast_slice(&model_matrices));

        app.presentation.present(|render_pass| {
            render_pass.set_pipeline(&pipeline);
            render_pass.set_bind_group(0, &textured_bind_group, &[]);

            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            //render_pass.draw_indexed(0..suzanne.indices.len() as u32, 0, 0..1);

            render_pass.draw_indexed(suzanne_range.clone(), suzanne_base_vertex, 0..1);
            render_pass.draw_indexed(cube_range.clone(), cube_base_vertex as i32, 1..2);
            render_pass.draw_indexed(sphere_range.clone(), sphere_base_vertex as i32, 2..3);
            //render_pass.draw_indexed(suzanne_range.clone(), suzanne_base_vertex, 2..3);
        })?;

        i += 0.1;

        Ok(())
    })?;

    Ok(())
}

