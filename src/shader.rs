use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Geometry,
    TessControl,
    TessEvaluation,
    Compute,
}

impl ShaderType {
    pub fn from_gl_enum(gl_enum: u32) -> ShaderType {
        match gl_enum {
            gl::VERTEX_SHADER => ShaderType::Vertex,
            gl::FRAGMENT_SHADER => ShaderType::Fragment,
            gl::GEOMETRY_SHADER => ShaderType::Geometry,
            gl::TESS_CONTROL_SHADER => ShaderType::TessControl,
            gl::TESS_EVALUATION_SHADER => ShaderType::TessEvaluation,
            gl::COMPUTE_SHADER => ShaderType::Compute,
            _ => panic!("Invalid gl enum for shader type: {}", gl_enum),
        }
    }

    pub fn to_gl_enum(&self) -> u32 {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER,
            ShaderType::Geometry => gl::GEOMETRY_SHADER,
            ShaderType::TessControl => gl::TESS_CONTROL_SHADER,
            ShaderType::TessEvaluation => gl::TESS_EVALUATION_SHADER,
            ShaderType::Compute => gl::COMPUTE_SHADER,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ShaderType::Vertex => "vertex",
            ShaderType::Fragment => "fragment",
            ShaderType::Geometry => "geometry",
            ShaderType::TessControl => "tess control",
            ShaderType::TessEvaluation => "tess evaluation",
            ShaderType::Compute => "compute",
        }
    }
}

pub struct Shader {
    id: u32,
    _type: ShaderType,
}

impl Shader {
    pub fn from_source(source: &str, _type: ShaderType) -> Result<Shader, String> {
        unsafe {
            let id = gl::CreateShader(_type.to_gl_enum());
            let source_ptr = source.as_ptr() as *const i8;
            let source_len = source.len() as i32;
            gl::ShaderSource(id, 1, &source_ptr, &source_len);
            gl::CompileShader(id);

            let mut success = 0;
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut info_log_length = 0;
                gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut info_log_length);
                let mut info_log = vec![0; info_log_length as usize];
                gl::GetShaderInfoLog(id, info_log_length, &mut info_log_length, info_log.as_mut_ptr() as *mut i8);
                let info_log = String::from_utf8(info_log).unwrap();
                return Err(info_log);
            }

            Ok(Shader { id, _type })
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

pub struct Program {
    id: u32,
    uniforms: HashMap<String, i32>,
}

impl Program {
    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        unsafe {
            let id = gl::CreateProgram();
            for shader in shaders {
                gl::AttachShader(id, shader.id);
            }
            gl::LinkProgram(id);

            let mut success = 0;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut info_log_length = 0;
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut info_log_length);
                let mut info_log = vec![0; info_log_length as usize];
                gl::GetProgramInfoLog(id, info_log_length, &mut info_log_length, info_log.as_mut_ptr() as *mut i8);
                let info_log = String::from_utf8(info_log).unwrap();
                return Err(info_log);
            }

            for shader in shaders {
                gl::DetachShader(id, shader.id);
            }

            Ok(Program { id })
        }
    }

    pub fn activate(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn deactivate() {
        unsafe {
            gl::UseProgram(0);
        }
    }

    pub fn activate_uniform(&mut self, name: &str) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, name.as_ptr() as _);
            self.uniforms.insert(name.to_string(), location);
        }
    }

    fn get_uniform_location(&self, name: &str) -> Result<i32, String> {
        self.uniforms.get(name).copied().ok_or_else(|| format!("Uniform {} not found", name))
    }

    pub fn set_uniform_1i(&self, name: &str, value: i32) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform1i(location, value);
        }
        Ok(())
    }

    pub fn set_uniform_1f(&self, name: &str, value: f32) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform1f(location, value);
        }
        Ok(())
    }

    pub fn set_uniform_2f(&self, name: &str, value: [f32; 2]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform2f(location, value[0], value[1]);
        }
        Ok(())
    }

    pub fn set_uniform_3f(&self, name: &str, value: [f32; 3]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform3f(location, value[0], value[1], value[2]);
        }
        Ok(())
    }

    pub fn set_uniform_4f(&self, name: &str, value: [f32; 4]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform4f(location, value[0], value[1], value[2], value[3]);
        }
        Ok(())
    }

    pub fn set_uniform_matrix_4f(&self, name: &str, value: &[f32; 16]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::UniformMatrix4fv(location, 1, gl::FALSE, value.as_ptr());
        }
        Ok(())
    }

    pub fn set_uniform_matrix_3f(&self, name: &str, value: &[f32; 9]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::UniformMatrix3fv(location, 1, gl::FALSE, value.as_ptr());
        }
        Ok(())
    }

    pub fn set_uniform_matrix_2f(&self, name: &str, value: &[f32; 4]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::UniformMatrix2fv(location, 1, gl::FALSE, value.as_ptr());
        }
        Ok(())
    }

    pub fn set_uniform_1iv(&self, name: &str, value: &[i32]) -> Result<(), String> {
        let location = self.get_uniform_location(name)?;
        unsafe {
            gl::Uniform1iv(location, value.len() as i32, value.as_ptr());
        }
        Ok(())
    }
}
