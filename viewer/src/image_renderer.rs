use std::path::Path;

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

        let r = ImageRenderer {
            program, vertex_array, buffer, texture,
            texture_loaded: false,
            texture_size: [0, 0],
        };

        r.set_scale([1.0, 1.0]);
        r.set_transform([0.0, 0.0]);

        r
    }

    pub fn render(&self) {
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

    pub fn set_texture_data<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let tex_data = load_texture(path, self.texture)?;
        self.texture_size = tex_data.size;
        self.texture_loaded = true;

        Ok(())
    }

    pub fn get_image_size(&self) -> [i32; 2] {
        self.texture_size
    }

    pub fn set_scale(&self, scale: [f32;2]) {
        unsafe {
            gl::UseProgram(self.program);
            let location = gl::GetUniformLocation(self.program,
                "scale\0".as_ptr() as _);
            gl::Uniform2f(location, scale[0], scale[1]);
        }
    }

    pub fn set_transform(&self, transform: [f32;2]) {
        unsafe {
            gl::UseProgram(self.program);
            let location = gl::GetUniformLocation(self.program,
                "transform\0".as_ptr() as _);
            gl::Uniform2f(location, transform[0], transform[1]);
        }
    }
}

impl std::ops::Drop for ImageRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
            gl::DeleteProgram(self.program);
            gl::DeleteTextures(1, &self.texture);
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
        \
        uniform vec2 scale;\n\
        uniform vec2 transform;\n\
        \
        void main() {\n\
            gl_Position = vec4(pos * scale + transform, 0.0, 1.0);\n\
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
