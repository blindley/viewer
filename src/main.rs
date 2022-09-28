fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();

    if args.len() < 1 {
        eprintln!("expected image filename");
        std::process::exit(-1);
    }

    
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
    .with_title("fuzzy pickles");
    
    let wc = glutin::ContextBuilder::new().build_windowed(wb, &el).unwrap();
    let wc = unsafe { wc.make_current().unwrap() };
    
    gl::load_with(|p| wc.get_proc_address(p) as *const _);
    
    let mut app_data = {
        let filename = args[0].clone();
        AppData::new(filename)
    };

    el.run(move |event, _, control_flow| {
        use glutin::event::{Event, WindowEvent, StartCause};
        use glutin::event_loop::ControlFlow;

        *control_flow = ControlFlow::WaitUntil(app_data.next_update_time);

        match event {
            Event::LoopDestroyed => return,

            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => {
                    app_data.next_update_time += app_data.time_between_updates;
                    *control_flow = ControlFlow::WaitUntil(app_data.next_update_time);

                    if let Ok(sig) = FileSignature::new(&app_data.image_path) {
                        if app_data.sig != sig {
                            app_data.sig = sig;
                            if app_data.reload_texture().is_ok() {
                                wc.window().request_redraw()
                            }
                        }
                    }
                },

                _ => (),
            },

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    wc.resize(physical_size);
                    app_data.resize_window([physical_size.width as _, physical_size.height as _]);
                },

                WindowEvent::KeyboardInput { input, .. } => {
                    use glutin::event::VirtualKeyCode::{Escape};
                    match input.virtual_keycode {
                        Some(Escape) => *control_flow = ControlFlow::Exit,
                        _ => (),
                    }
                },

                _ => (),
            },
            Event::RedrawRequested(_) => {
                app_data.redraw();
                wc.swap_buffers().unwrap();
            },

            _ => (),
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    len: u64,
}

impl FileSignature {
    fn new<P: AsRef<std::path::Path>>(path: P) -> Result<FileSignature, Box<dyn std::error::Error>> {
        let mdata = std::fs::metadata(path)?;
        Ok(FileSignature {
            modified: mdata.modified().ok(),
            created: mdata.created().ok(),
            len: mdata.len(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct GLData {
    program: u32,
    vertex_array: u32,
    draw_mode: gl::types::GLenum,
    vertex_count: i32,
    texture: u32,
    buffer: u32,
}

#[derive(Debug, Clone)]
struct AppData {
    image_path: std::path::PathBuf,
    sig: FileSignature,
    window_size: [i32;2],
    image_size: [i32;2],
    next_update_time: std::time::Instant,
    time_between_updates: std::time::Duration,
    gl_data: GLData,
}

impl AppData {
    fn new<P: Into<std::path::PathBuf>>(image_path: P) -> AppData {
        let program = create_program();
        let texture = create_texture();
        let vertex_array = create_vertex_array();

        let time_between_updates = std::time::Duration::new(1, 0);
        let image_path = image_path.into();
        let sig = FileSignature::new(&image_path).unwrap();
    
        let mut app_data = AppData {
            image_path: image_path,
            sig: sig,
            window_size: [1,1],
            image_size: [1,1],
            time_between_updates: time_between_updates,
            next_update_time: std::time::Instant::now() + time_between_updates,
            gl_data: GLData {
                program: program,
                vertex_array: vertex_array.vertex_array,
                draw_mode: vertex_array.draw_mode,
                vertex_count: vertex_array.vertex_count,
                buffer: vertex_array.buffer,
                texture: texture,
            },
        };
    
        if let Err(_) = app_data.reload_texture() {
            eprintln!("failed to load {:?}", app_data.image_path);
            std::process::exit(-1);
        }

        app_data
    }

    fn redraw(&self) {
        unsafe {
            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.gl_data.texture);

            gl::UseProgram(self.gl_data.program);

            gl::BindVertexArray(self.gl_data.vertex_array);
            gl::DrawArrays(self.gl_data.draw_mode, 0, self.gl_data.vertex_count);
        }
    }

    fn reload_texture(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.image_size =
            load_texture(&self.image_path, self.gl_data.texture)?
            .size;

        self.recalculate_aspect_ratio();
        Ok(())
    }

    fn resize_window(&mut self, size: [i32;2]) {
        self.window_size = size;
        self.recalculate_aspect_ratio();
        
        unsafe { gl::Viewport(0, 0, size[0], size[1]); }
    }

    fn recalculate_aspect_ratio(&self) {
        let window_aspect_ratio =
            (self.window_size[0] as f32) / (self.window_size[1] as f32);
        let image_aspect_ratio =
            (self.image_size[0] as f32) / (self.image_size[1] as f32);
        let aspect_ratio = image_aspect_ratio / window_aspect_ratio;

        unsafe {
            gl::UseProgram(self.gl_data.program);
            let location = gl::GetUniformLocation(self.gl_data.program,
                "aspect_ratio\0".as_ptr() as _);
            gl::Uniform1f(location, aspect_ratio);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TextureMetadata {
    size: [i32;2],
}

fn load_texture<P: AsRef<std::path::Path>>(filename: P, texture_id: u32)
-> Result<TextureMetadata, Box<dyn std::error::Error>>
{
    unsafe {
        let img = image::open(filename)?
            .into_rgba8();

        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        let data = img.as_ptr() as _;
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _,
            img.width() as _, img.height() as _,
            0, gl::RGBA, gl::UNSIGNED_BYTE, data);

        gl::GenerateMipmap(gl::TEXTURE_2D);

        let size = [img.width() as i32, img.height() as i32];

        Ok(TextureMetadata { size })
    }
}

pub struct BufferData {
    #[allow(dead_code)]
    buffer: u32,
    vertex_array: u32,
    draw_mode: gl::types::GLenum,
    vertex_count: i32,
}

fn create_vertex_array() -> BufferData {
    unsafe {
        let (mut buffer, mut vertex_array) = (0, 0);

        let vertices = [
            // position  // tex coords
            -1.0,  1.0,  0.0, 0.0,     // top left 
             1.0,  1.0,  1.0, 0.0,     // top right
             1.0, -1.0,  1.0, 1.0,     // bottom right
            -1.0, -1.0,  0.0, 1.0,     // bottom left
        ];

        let _: f32 = vertices[0]; // dumb hack to force vertices to be array of f32

        gl::GenVertexArrays(1, &mut vertex_array);
        gl::GenBuffers(1, &mut buffer);

        gl::BindVertexArray(vertex_array);

        gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
        let size = std::mem::size_of_val(&vertices) as _;
        let ptr = vertices.as_ptr() as _;
        gl::BufferData(gl::ARRAY_BUFFER, size, ptr, gl::STATIC_DRAW);

        let stride = (4 * std::mem::size_of::<f32>()) as _;

        let ptr = 0 as _;
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, ptr);
        gl::EnableVertexAttribArray(0);

        let ptr = (2 * std::mem::size_of::<f32>()) as _;
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, ptr);
        gl::EnableVertexAttribArray(1);

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);

        gl::BindVertexArray(0);

        let draw_mode = gl::TRIANGLE_FAN;
        let vertex_count = 4;

        BufferData {
            buffer,
            vertex_array,
            draw_mode,
            vertex_count,
        }
    }
}

fn create_texture() -> u32 {
    unsafe {
        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

        texture
    }
}

fn create_program() -> u32 {
    unsafe {
        let vshader = compile_shader(shader_code::VERTEX_SHADER_SOURCE, gl::VERTEX_SHADER);
        let fshader = compile_shader(shader_code::FRAGMENT_SHADER_SOURCE, gl::FRAGMENT_SHADER);

        let program = gl::CreateProgram();
        gl::AttachShader(program, vshader);
        gl::AttachShader(program, fshader);
        gl::LinkProgram(program);
        
        gl::DeleteShader(vshader);
        gl::DeleteShader(fshader);

        gl::UseProgram(program);
        let location = gl::GetUniformLocation(program, "texture1\0".as_ptr() as _); 
        gl::Uniform1i(location, 0);

        let location = gl::GetUniformLocation(program, "aspect_ratio\0".as_ptr() as _);
        gl::Uniform1f(location, 1.0);

        program
    }
}

fn compile_shader(code: &str, type_: gl::types::GLenum) -> u32 {
    unsafe {
        let code_ptr = code.as_ptr() as _;
        let shader = gl::CreateShader(type_);
        gl::ShaderSource(shader, 1, &code_ptr, 0 as _);
        gl::CompileShader(shader);

        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);

        if success == 0 {
            let mut info_log_buffer = [0i8;512];
            let ptr = info_log_buffer.as_mut_ptr();
            gl::GetShaderInfoLog(shader, 512, 0 as _, ptr);

            let cstr = std::ffi::CStr::from_ptr(ptr);
            let str_slice = cstr.to_str().unwrap();
            panic!("{}", str_slice);
        }

        shader
    }
}

mod shader_code {
    pub const VERTEX_SHADER_SOURCE: &str =
        "\
        #version 330 core\n\
        layout (location = 0) in vec2 pos;\n\
        layout (location = 1) in vec2 tcoords;\n\
        \
        out vec2 vtcoords;\n\
        uniform float aspect_ratio;\n\
        \
        void main() {\n\
            float horz = 1.0f;
            float vert = 1.0f / aspect_ratio;
            if (aspect_ratio < 1.0f) {
                vert = 1.0f;
                horz = aspect_ratio;
            }
            gl_Position = vec4(pos.x * horz, pos.y * vert, 0.0, 1.0);\n\
            vtcoords = tcoords;
        }\n\
        \0";

    pub const FRAGMENT_SHADER_SOURCE: &str =
        "\
        #version 330 core\n\
        in vec2 vtcoords;\n\
        out vec4 fcolor;\n\
        \
        uniform sampler2D texture1;\n\
        \
        void main() {\n\
            fcolor = texture(texture1, vtcoords);\n\
        }\n\
        \0";

}
