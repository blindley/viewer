use crate::texture::{Texture, create_texture};

pub trait Renderer {
    fn render(&self);
    fn set_scale(&mut self, scale: [f32;2]);
    fn set_translate(&mut self, translate: [f32;2]);
}

#[derive(Debug)]
pub struct ImageRenderer {
    program: u32,
    vertex_array: u32,
    buffer: u32,
    texture: u32,

    texture_loaded: bool,
    texture_size: [i32; 2],
}

impl ImageRenderer {
    pub fn new() -> ImageRenderer {
        let program = create_program();
        let texture = create_texture();
        let BufferData { buffer, vertex_array } = create_vertex_array();

        let mut r = ImageRenderer {
            program, vertex_array, buffer, texture,
            texture_loaded: false,
            texture_size: [0, 0],
        };

        r.set_scale([1.0, 1.0]);
        r.set_translate([0.0, 0.0]);

        r
    }

    pub fn set_texture_data(&mut self, texture: &Texture) -> Result<(), Box<dyn std::error::Error>> {
        self.texture = texture.texture_id;
        self.texture_size = texture.size;
        self.texture_loaded = true;

        Ok(())
    }

    pub fn get_image_size(&self) -> [i32; 2] {
        self.texture_size
    }
}

impl std::ops::Drop for ImageRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteProgram(self.program);
        }
    }
}

impl Renderer for ImageRenderer {
    fn render(&self) {
        if self.texture_loaded {
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.texture);

                gl::UseProgram(self.program);

                gl::BindVertexArray(self.vertex_array);
                gl::DrawArrays(gl::TRIANGLE_FAN, 0, 4);
            }
        }
    }

    fn set_scale(&mut self, scale: [f32;2]) {
        unsafe {
            gl::UseProgram(self.program);
            let location = gl::GetUniformLocation(self.program,
                b"scale\0".as_ptr() as _);
            gl::Uniform2f(location, scale[0], scale[1]);
        }
    }

    fn set_translate(&mut self, translate: [f32;2]) {
        unsafe {
            gl::UseProgram(self.program);
            let location = gl::GetUniformLocation(self.program,
                b"translate\0".as_ptr() as _);
            gl::Uniform2f(location, translate[0], translate[1]);
        }
    }
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
        \
        uniform vec2 scale;\n\
        uniform vec2 translate;\n\
        \
        void main() {\n\
            gl_Position = vec4(pos * scale + translate, 0.0, 1.0);\n\
            vtcoords = tcoords;\n\
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
