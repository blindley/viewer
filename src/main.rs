fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();

    if args.len() < 1 {
        return;
    }

    let filename = args[0].clone();
    let mdata = std::fs::metadata(&filename).unwrap();
    let mut last_write_time = mdata.modified().unwrap();

    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("fuzzy pickles");
    
    let wc = glutin::ContextBuilder::new().build_windowed(wb, &el).unwrap();
    let wc = unsafe { wc.make_current().unwrap() };
    
    gl::load_with(|p| wc.get_proc_address(p) as *const _);

    
    let program = create_program();
    
    let image_size;
    let mut texture1 = 0;
    unsafe {
        gl::GenTextures(1, &mut texture1);
        gl::BindTexture(gl::TEXTURE_2D, texture1);
        
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

        let metadata = load_texture(&filename, texture1);
        
        let img = load_image(&filename);
        
        let data = img.as_ptr() as _;
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _, img.width() as _, img.height() as _,
            0, gl::RGBA, gl::UNSIGNED_BYTE, data);
        
        gl::GenerateMipmap(gl::TEXTURE_2D);
        
        gl::UseProgram(program);
        let location = gl::GetUniformLocation(program, "texture1\0".as_ptr() as _); 
        gl::Uniform1i(location, 0);

        image_size = [img.width() as i32, img.height() as i32];
        
        let location = gl::GetUniformLocation(program, "aspect_ratio\0".as_ptr() as _);
        gl::Uniform1f(location, 1.0);
    }
    
    let vertex_array = create_vertex_array();

    let mut app_data = AppData {
        image_filename: filename,
        window_size: [1,1],
        image_size: image_size,
        gl_data: GLData {
            program: program,
            vertex_array: vertex_array.vertex_array,
            buffer: vertex_array.buffer,
            texture: texture1,
        },
    };

    let mut next_check_time = std::time::Instant::now() + std::time::Duration::new(2, 0);

    el.run(move |event, _, control_flow| {
        use glutin::event::{Event, WindowEvent, StartCause};
        use glutin::event_loop::ControlFlow;

        *control_flow = ControlFlow::WaitUntil(next_check_time);
        match event {
            Event::LoopDestroyed => return,

            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => {
                    use std::time::{Instant, Duration};
                    next_check_time = Instant::now() + Duration::new(1, 0);
                    *control_flow = ControlFlow::WaitUntil(next_check_time);

                    let mdata = std::fs::metadata(&app_data.image_filename).unwrap();
                    let new_last_write_time = mdata.modified().unwrap();
                    if last_write_time < new_last_write_time {
                        last_write_time = new_last_write_time;
                        let img = load_image(&app_data.image_filename);

                        unsafe {

                            gl::BindTexture(gl::TEXTURE_2D, app_data.gl_data.texture);

                            let data = img.as_ptr() as _;
                            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _,
                                img.width() as _, img.height() as _,
                                0, gl::RGBA, gl::UNSIGNED_BYTE, data);
            
                            gl::GenerateMipmap(gl::TEXTURE_2D);

                            app_data.image_size = [img.width() as i32, img.height() as i32];

                            let window_aspect_ratio =
                                (app_data.window_size[0] as f32) / (app_data.window_size[1] as f32);
                            let image_aspect_ratio =
                                (app_data.image_size[0] as f32) / (app_data.image_size[1] as f32);
                            let aspect_ratio = image_aspect_ratio / window_aspect_ratio;

                            gl::UseProgram(app_data.gl_data.program);
                            let location = gl::GetUniformLocation(app_data.gl_data.program,
                                "aspect_ratio\0".as_ptr() as _);
                            gl::Uniform1f(location, aspect_ratio);
                        }

                        wc.window().request_redraw();
                    }
                },

                _ => (),
            },

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    wc.resize(physical_size);
                    unsafe {
                        let width = physical_size.width;
                        let height = physical_size.height;
                        gl::Viewport(0, 0, width as _, height as _);

                        app_data.window_size = [width as i32, height as i32];

                        let window_aspect_ratio = (width as f32) / (height as f32);
                        let image_aspect_ratio =
                            (app_data.image_size[0] as f32) / (app_data.image_size[1] as f32);
                        let aspect_ratio = image_aspect_ratio / window_aspect_ratio;

                        gl::UseProgram(app_data.gl_data.program);
                        let location = gl::GetUniformLocation(app_data.gl_data.program,
                            "aspect_ratio\0".as_ptr() as _);
                        gl::Uniform1f(location, aspect_ratio);
                    }
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
                unsafe {
                    gl::ClearColor(0.1, 0.1, 0.1, 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT);

                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, app_data.gl_data.texture);

                    gl::UseProgram(app_data.gl_data.program);

                    gl::BindVertexArray(app_data.gl_data.vertex_array);
                    gl::DrawArrays(gl::TRIANGLES, 0, 6);
                }

                wc.swap_buffers().unwrap();
            },

            _ => (),
        }
    });
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct GLData {
    program: u32,
    vertex_array: u32,
    texture: u32,
    buffer: u32,
}

#[derive(Debug, Clone)]
struct AppData {
    image_filename: String,
    window_size: [i32;2],
    image_size: [i32;2],
    gl_data: GLData,
}

#[derive(Debug, Clone, Copy)]
struct TextureMetadata {
    size: [i32;2],
}

fn load_texture<P: AsRef<std::path::Path>>(filename: P, texture_id: u32) -> TextureMetadata {
    unsafe {
        let img = image::open(filename).unwrap().into_rgba8();

        gl::BindTexture(gl::TEXTURE_2D, texture_id);

        let data = img.as_ptr() as _;
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _,
            img.width() as _, img.height() as _,
            0, gl::RGBA, gl::UNSIGNED_BYTE, data);

        gl::GenerateMipmap(gl::TEXTURE_2D);

        let size = [img.width() as i32, img.height() as i32];

        TextureMetadata { size }
    }
}

fn load_image<P: AsRef<std::path::Path>> (filename: P) -> image::RgbaImage {
    #[allow(unused_imports)]
    use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage};

    let img = image::open(filename).unwrap().into_rgba8();

    img
}

pub struct BufferData {
    #[allow(dead_code)]
    buffer: u32,
    vertex_array: u32,
}

fn create_vertex_array() -> BufferData {
    unsafe {
        let (mut buffer, mut vertex_array) = (0, 0);

        let vertices = [
            // position  // tex coords
            -1.0,  1.0,  0.0, 0.0,     // top left 
             1.0,  1.0,  1.0, 0.0,     // top right
            -1.0, -1.0,  0.0, 1.0,     // bottom left
            -1.0, -1.0,  0.0, 1.0,     // bottom left
             1.0,  1.0,  1.0, 0.0,     // top right
             1.0, -1.0,  1.0, 1.0,     // bottom right
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

        BufferData {
            buffer,
            vertex_array,
        }
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
        
        gl::UseProgram(program);
        
        gl::DeleteShader(vshader);
        gl::DeleteShader(fshader);

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
