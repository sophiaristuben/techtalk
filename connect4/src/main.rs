use assets_manager::{source, AssetCache};
use bytemuck::{Pod, Zeroable};
use image::{DynamicImage, GenericImageView};
use std::{borrow::Cow, mem};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod input;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct GPUCamera {
    screen_pos: [f32; 2],
    screen_size: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct GPUSprite {
    screen_region: [f32; 4],
    sheet_region: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SpriteOption {
    Storage,
    Uniform,
    VertexBuffer,
}

#[derive(Copy, Clone)]
struct Space {
    color: &'static str, // Use a string slice for color
    filled: bool,
}

impl Space {
    fn new(color: &'static str) -> Self {
        Space {
            color,
            filled: false,
        }
    }
}

struct GameGrid {
    grid: [[Space; 7]; 6],
}

impl GameGrid {
    fn new() -> Self {
        let mut grid = [[Space::new("white"); 7]; 6];

        // Initialize the grid with space objects
        for row in 0..6 {
            for col in 0..7 {
                grid[row][col] = Space::new("None");
            }
        }

        GameGrid { grid }
    }

    fn print_grid(&self) {
        for row in &self.grid {
            for space in row {
                if space.filled {
                    print!("1 "); // You can change this to any character or representation for filled spaces
                } else {
                    print!("0 "); // You can change this to any character or representation for empty spaces
                }
            }
            println!();
        }
    }

    fn fill_space(&mut self, x: usize, y: usize, color: &'static str,) {
        if x < 7 && y < 6 {
            self.grid[y][x].filled = true;
            self.grid[y][x].color = color;
        }
    }

    fn check_win(&self) -> (bool, &str) {
        // Check horizontally
        for row in &self.grid {
            let mut consecutive_count = 0;
            let mut last_color = "";

            for space in row {
                if space.filled && space.color == last_color {
                    consecutive_count += 1;
                    if consecutive_count == 4 {
                        return (true, space.color); // Four consecutive spaces found horizontally
                    }
                } else {
                    consecutive_count = 1;
                    last_color = space.color;
                }
            }
        }

        // Check vertically
        for col in 0..7 {
            let mut consecutive_count = 0;
            let mut last_color = "";

            for row in 0..6 {
                let space = &self.grid[row][col];

                if space.filled && space.color == last_color {
                    consecutive_count += 1;
                    if consecutive_count == 4 {
                        return (true, space.color); // Four consecutive spaces found vertically
                    }
                } else {
                    consecutive_count = 1;
                    last_color = space.color;
                }
            }
        }

        // Check diagonally (top-left to bottom-right)
        for start_row in 0..3 {
            for start_col in 0..4 {
                let mut consecutive_count = 0;
                let mut last_color = "";

                for step in 0..4 {
                    let row = start_row + step;
                    let col = start_col + step;

                    let space = &self.grid[row][col];

                    if space.filled && space.color == last_color {
                        consecutive_count += 1;
                        if consecutive_count == 4 {
                            return (true, space.color); // Four consecutive spaces found diagonally
                        }
                    } else {
                        consecutive_count = 1;
                        last_color = space.color;
                    }
                }
            }
        }

        // Check diagonally (top-right to bottom-left)
        for start_row in 0..3 {
            for start_col in 3..7 {
                let mut consecutive_count = 0;
                let mut last_color = "";

                for step in 0..4 {
                    let row = start_row + step;
                    let col = start_col - step;

                    let space = &self.grid[row][col];

                    if space.filled && space.color == last_color {
                        consecutive_count += 1;
                        if consecutive_count == 4 {
                            return (true, space.color); // Four consecutive spaces found diagonally
                        }
                    } else {
                        consecutive_count = 1;
                        last_color = space.color;
                    }
                }
            }
        }
        return (false, &"") // No four consecutive spaces found
    }
}

#[cfg(all(not(feature = "uniforms"), not(feature = "vbuf")))]
const SPRITES: SpriteOption = SpriteOption::Storage;
#[cfg(feature = "uniforms")]
const SPRITES: SpriteOption = SpriteOption::Uniform;
#[cfg(feature = "vbuf")]
const SPRITES: SpriteOption = SpriteOption::VertexBuffer;
#[cfg(all(feature = "vbuf", feature = "uniform"))]
compile_error!("Can't choose both vbuf and uniform sprite features");

async fn run(event_loop: EventLoop<()>, window: Window, cache: AssetCache) {
    let size = window.inner_size();

    log::info!("Use sprite mode {:?}", SPRITES);

    let instance = wgpu::Instance::default();

    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: if SPRITES == SpriteOption::Storage {
                    wgpu::Limits::downlevel_defaults()
                } else {
                    wgpu::Limits::downlevel_webgl2_defaults()
                }
                .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    if SPRITES == SpriteOption::Storage {
        let supports_storage_resources = adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::VERTEX_STORAGE)
            && device.limits().max_storage_buffers_per_shader_stage > 0;
        assert!(supports_storage_resources, "Storage buffers not supported");
    }
    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let shader2 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader2.wgsl"))),
    });

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            // It needs the first entry for the texture and the second for the sampler.
            // This is like defining a type signature.
            entries: &[
                // The texture binding
                wgpu::BindGroupLayoutEntry {
                    // This matches the binding in the shader
                    binding: 0,
                    // Only available in the fragment shader
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // It's a texture binding
                    ty: wgpu::BindingType::Texture {
                        // We can use it with float samplers
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        // It's being used as a 2D texture
                        view_dimension: wgpu::TextureViewDimension::D2,
                        // This is not a multisampled texture
                        multisampled: false,
                    },
                    count: None,
                },
                // The sampler binding
                wgpu::BindGroupLayoutEntry {
                    // This matches the binding in the shader
                    binding: 1,
                    // Only available in the fragment shader
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // It's a sampler
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    // No count
                    count: None,
                },
            ],
        });

    // The camera binding
    let camera_layout_entry = wgpu::BindGroupLayoutEntry {
        // This matches the binding in the shader
        binding: 0,
        // Available in vertex shader
        visibility: wgpu::ShaderStages::VERTEX,
        // It's a buffer
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        // No count, not a buffer array binding
        count: None,
    };
    let sprite_bind_group_layout = match SPRITES {
        SpriteOption::Storage => {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    camera_layout_entry,
                    wgpu::BindGroupLayoutEntry {
                        // This matches the binding in the shader
                        binding: 1,
                        // Available in vertex shader
                        visibility: wgpu::ShaderStages::VERTEX,
                        // It's a buffer
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        // No count, not a buffer array binding
                        count: None,
                    },
                ],
            })
        }
        SpriteOption::Uniform => {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    camera_layout_entry,
                    wgpu::BindGroupLayoutEntry {
                        // This matches the binding in the shader
                        binding: 1,
                        // Available in vertex shader
                        visibility: wgpu::ShaderStages::VERTEX,
                        // It's a buffer
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(SPRITE_UNIFORM_SIZE),
                        },
                        // No count, not a buffer array binding
                        count: None,
                    },
                ],
            })
        }
        SpriteOption::VertexBuffer => {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[camera_layout_entry],
            })
        }
    };
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&sprite_bind_group_layout, &texture_bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline_layout_over = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&texture_bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: match SPRITES {
                SpriteOption::Storage => "vs_storage_main",
                SpriteOption::Uniform => "vs_uniform_main",
                SpriteOption::VertexBuffer => "vs_vbuf_main",
            },
            buffers: match SPRITES {
                SpriteOption::VertexBuffer => &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GPUSprite>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: std::mem::size_of::<[f32; 4]>() as u64,
                            shader_location: 1,
                        },
                    ],
                }],
                _ => &[],
            },
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let render_pipeline_over = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout_over),
        vertex: wgpu::VertexState {
            module: &shader2,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader2,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    // let sprite = cache.load::<assets_manager::asset::Gltf>("content/connect4v2.png")
    // .unwrap();
    // let sprite_img = sprite.get_image_by_index(0);
    // let sprite_tex = ;
    // // frend.gpu.create_array_texture(
    // //     &[&fox_img.to_rgba8()],
    // //     frenderer::wgpu::TextureFormat::Rgba8Unorm,
    // //     (fox_img.width(), fox_img.height()),
    // //     Some("fox texture"),
    // // );
    let sprite = cache
    .load::<assets_manager::asset::Png>("connect4v2")
    .unwrap();
    let (sprite_tex, _sprite_img) = load_texture(&sprite.read().0, None, &device, &queue).unwrap();
    let view_sprite = sprite_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler_sprite = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &texture_bind_group_layout,
        entries: &[
            // One for the texture, one for the sampler
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_sprite),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler_sprite),
            },
        ],
    });

    let camera = GPUCamera {
        screen_pos: [0.0, 0.0],
        screen_size: [724.0, 650.0],
    };
    let buffer_camera = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytemuck::bytes_of(&camera).len() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut sprites: Vec<GPUSprite> = vec![ 
        // these sprites initial locations are determined by sprite_position_x
        // screen_region [x,y,z,w] = top left corner x, top left corner y, width, height
        // sheet_region [x,y,z,w] = divided by spritesheet width, divided by spritesheet height, divided by spritesheet width, divided by spritesheet height, divided by spritesheet width, divided by spritesheet height,
        GPUSprite {
            screen_region: [0.0, 0.0, 724.0, 650.0],
            sheet_region: [0.0, 0.0 / 810.0, 724.0 / 724.0, 650.0 / 810.0],
        }, 
    ];

    for _ in 0..21 {
        let sprite = GPUSprite {
            screen_region: [0.0, 651.0, 70.0, 67.0],
            sheet_region: [0.0 / 724.0, 651.0 / 810.0, 70.0 / 724.0, 67.0/ 810.0]
        };
        sprites.push(sprite);
        let sprite2 = GPUSprite {
            screen_region: [0.0, 720.0, 70.0, 67.0],
            sheet_region: [0.0 / 724.0, 720.0 / 810.0, 70.0 / 724.0, 67.0/ 810.0]
        };
        sprites.push(sprite2);
    }

    // here divide by a number to create the number of grids
    let cell_width = 103.0 ;

    // Initialize sprite positions within the grid
    let mut sprite_position: [f32; 2] = [330.0, 575.0];

    // current sprite
    let mut curr_sprite_index = 1;
    let mut placed_cells = 1;

    const SPRITE_UNIFORM_SIZE: u64 = 512 * mem::size_of::<GPUSprite>() as u64;

    let buffer_sprite = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: if SPRITES == SpriteOption::Uniform {
            SPRITE_UNIFORM_SIZE
        } else {
            sprites.len() as u64 * std::mem::size_of::<GPUSprite>() as u64
        },
        usage: match SPRITES {
            SpriteOption::Storage => wgpu::BufferUsages::STORAGE,
            SpriteOption::Uniform => wgpu::BufferUsages::UNIFORM,
            SpriteOption::VertexBuffer => wgpu::BufferUsages::VERTEX,
        } | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let sprite_bind_group = match SPRITES {
        SpriteOption::VertexBuffer => device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sprite_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer_camera.as_entire_binding(),
            }],
        }),
        _ => device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer_camera.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_sprite.as_entire_binding(),
                },
            ],
        }),
    };

    queue.write_buffer(&buffer_camera, 0, bytemuck::bytes_of(&camera));
    queue.write_buffer(&buffer_sprite, 0, bytemuck::cast_slice(&sprites));

    let mut input = input::Input::default();
    let mut game_grid = GameGrid::new();
    let mut show_end_screen = false;
    let mut win_color = "".to_string();


    let yellow = cache
    .load::<assets_manager::asset::Png>("connect4v2")
    .unwrap();
    let (tex_yellow, _win_img) = load_texture(&yellow.read().0, None, &device, &queue).unwrap();
    // let (tex_yellow, _win_image) = load_texture("content/yellowWins.png",None, &device, &queue)
    //     .await
    //     .expect("Couldn't load game over img");

    let red = cache
    .load::<assets_manager::asset::Png>("connect4v2")
    .unwrap();
    let (tex_red, _win_img) = load_texture(&red.read().0, None, &device, &queue).unwrap();
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                
                // handles the left movement of the chip
                if input.is_key_pressed(winit::event::VirtualKeyCode::Left) {
                    let mut x_occupied = false;
                    for curr in 1..curr_sprite_index {
                        let x = sprites[curr].screen_region[0];
                        let y = sprites[curr].screen_region[1];
                        if x == sprite_position[0]-cell_width && y == sprite_position[1] {
                            // Update the screen_region of the current sprite
                            x_occupied = true;
                        } 
                    }
                    if sprite_position[0] <= 124.0 {
                        sprite_position[0] = 21.0;
                    } 
                    else if !x_occupied {
                        sprite_position[0] -= cell_width; 
                    }
                }

                // handles the right movement 
                if input.is_key_pressed(winit::event::VirtualKeyCode::Right) {
                    let mut x_occupied = false;
                    for curr in 1..curr_sprite_index {
                        let x = sprites[curr].screen_region[0];
                        let y = sprites[curr].screen_region[1];
                        if x == sprite_position[0] + cell_width && y == sprite_position[1] {
                            x_occupied = true;
                        } 
                    }

                    if sprite_position[0] > 536.0 {
                        sprite_position[0] = 638.0;
                    } else if !x_occupied {
                        sprite_position[0] += cell_width;
                    }
                }

                if input.is_key_pressed(winit::event::VirtualKeyCode::Down) {
                    sprite_position[1] -= 81.0;
                }

                if input.is_key_pressed(winit::event::VirtualKeyCode::Up) {
                    game_grid.print_grid();
                    let my_bool = game_grid.check_win();
                    println!("{:?}", my_bool)
                }

                //update sprite position based in key presses
                sprites[curr_sprite_index].screen_region[0] = sprite_position[0];
                sprites[curr_sprite_index].screen_region[1] = sprite_position[1];
                
                // get the current location
                let curr_x = sprites[curr_sprite_index].screen_region[0];
                let curr_y = sprites[curr_sprite_index].screen_region[1];
                let mut collision = false;
                let mut y_being_checked = 0.0;

                //  check if the current location has a sprite in it by looping through coins up to the current coin
                for curr in 1..curr_sprite_index {
                    let x = sprites[curr].screen_region[0];
                    let y = sprites[curr].screen_region[1];

                    if x == curr_x && y == curr_y {
                        y_being_checked = y;
                        collision = true;
                    } 
                }


                if collision {
                    println!("collision!");
                    //update sprite position to be 88px above sprite location
                    sprites[curr_sprite_index].screen_region[1] = y_being_checked + 81.0;
                    sprite_position[1] += 81.0;

                    // left x coordinate plus half the width selects the center of the sprite 
                    let grid_x = (sprites[curr_sprite_index].screen_region[0] as usize + sprites[curr_sprite_index].screen_region[2] as usize / 2) / 104;
                    let grid_y = 5 - (sprites[curr_sprite_index].screen_region[1] as usize - 89) / 81;

                    // if the piece is red, mark the corresponding spot in the game_grid as filled with yellow
                    if curr_sprite_index % 2 == 0 {
                        game_grid.fill_space(grid_x, grid_y,  "yellow");
                    } else{
                        game_grid.fill_space(grid_x, grid_y,  "red");
                    }

                    // move onto the next sprite
                    curr_sprite_index += 1;
                    // move the cell pointer one forward to mark that another has been added to the screen
                    placed_cells += 1;

                    // this code ensures that the next sprite rendered shows up in the center top of the screen
                    sprite_position[0] = 330.0;
                    sprite_position[1] = 575.0;

                    let (over, winning_color) = game_grid.check_win();
                    if over {
                        println!("{} wins!", winning_color);
                        show_end_screen = true;
                        win_color = winning_color.to_string();
                    }

                // check if the piece has hit the bottom of the screen
                } else if sprite_position[1] <= 89.0 {
                    println!("{}", "bottom!");

                    // set the current sprite's y to the bottom of the screen
                    sprites[curr_sprite_index].screen_region[1] = 89.0;

                    // left x coordinate plus half the width selects the center of the sprite 
                    let grid_x = (sprites[curr_sprite_index].screen_region[0] as usize + sprites[curr_sprite_index].screen_region[2] as usize / 2) / 104;
                    let grid_y = 5 - (sprites[curr_sprite_index].screen_region[1] as usize - 89) / 81;

                    // if the piece is red, mark the corresponding spot in the game_grid as filled with yellow
                    if curr_sprite_index % 2 == 0 {
                        game_grid.fill_space(grid_x, grid_y,  "yellow");
                    } else{
                        game_grid.fill_space(grid_x, grid_y,  "red");
                    }

                    // move onto the next sprite 
                    curr_sprite_index += 1;
                    // move the cell pointer one forward to mark that another has been added to the screen
                    placed_cells += 1;
                    
                    sprite_position[0] = 330.0;
                    sprite_position[1] = 575.0;

                    let (over, winning_color) = game_grid.check_win();
                    if over {
                        println!("{} wins!", winning_color);
                        show_end_screen = true;
                        win_color = winning_color.to_string();
                    }

                } 

                // Then send the data to the GPU!
                input.next_frame();

                queue.write_buffer(&buffer_camera, 0, bytemuck::bytes_of(&camera));
                queue.write_buffer(&buffer_sprite, 0, bytemuck::cast_slice(&sprites));

                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    if show_end_screen{
                        let tex_end = 
                        if win_color == "yellow" {
                            &tex_yellow
                        } else {
                            &tex_red
                        };
                        
                        let view_end = tex_end.create_view(&wgpu::TextureViewDescriptor::default());
                        let sampler_end = device.create_sampler(&wgpu::SamplerDescriptor::default());
                        
                        let tex_over_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: None,
                            layout: &texture_bind_group_layout,
                            entries: &[
                                // One for the texture, one for the sampler
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: wgpu::BindingResource::TextureView(&view_end),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::Sampler(&sampler_end),
                                },
                            ],
                        });
                        
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                    store: true,
                                },
                            })],
                            depth_stencil_attachment: None,
                        });
                        // draw game over sprite
                        rpass.set_pipeline(&render_pipeline_over);
                        // Attach the bind group for group 0
                        rpass.set_bind_group(0, &tex_over_bind_group, &[]);
                        // Now draw two triangles!
                        rpass.draw(0..6, 0..1);

                    }

                    else {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                    store: true,
                                },
                            })],
                            depth_stencil_attachment: None,
                        });
    
    
                        
                        rpass.set_pipeline(&render_pipeline);
                        if SPRITES == SpriteOption::VertexBuffer {
                            rpass.set_vertex_buffer(0, buffer_sprite.slice(..));
                        }
                        rpass.set_bind_group(0, &sprite_bind_group, &[]);
                        rpass.set_bind_group(1, &texture_bind_group, &[]);
                        // draw the current sprite
                        rpass.draw(0..6, (curr_sprite_index as u32)..(curr_sprite_index as u32)+1);
                        // draw all the sprites that have already been placed
                        rpass.draw(0..6, 1..(placed_cells as u32));
                        // draw the connect 4 grid
                        rpass.draw(0..6, 0..1);

                    }
                            
                }
                
                queue.submit(Some(encoder.finish()));
                frame.present();
                window.request_redraw();
                
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            // WindowEvent->KeyboardInput: Keyboard input!
            Event::WindowEvent {
                // Note this deeply nested pattern match
                event: WindowEvent::KeyboardInput { input: key_ev, .. },
                ..
            } => {
                input.handle_key_event(key_ev);
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                input.handle_mouse_button(state, button);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                input.handle_mouse_move(position);
            }
            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let source = assets_manager::source::FileSystem::new("./content").unwrap();
        let cache = AssetCache::with_source(source);
        env_logger::init();
        pollster::block_on(run(event_loop, window, cache));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Trace).expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;

        let source = assets_manager::source::Embedded::from(source::embed!("./content"));
        let cache = AssetCache::with_source(source);


        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window, cache));
    }
}

fn load_texture(
    image: &DynamicImage,
    label: Option<&str>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(wgpu::Texture, image::RgbaImage), Box<dyn std::error::Error>> {
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    Ok((texture, image))
}