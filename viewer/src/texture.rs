use std::path::Path;

#[derive(Debug)]
pub struct Texture {
    pub texture_id: u32,
    pub size: [i32; 2],
}

impl Texture {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Texture, Box<dyn std::error::Error>> {
        let texture_id = create_texture();
        let tex_data = load_texture(path, texture_id)?;
        Ok(Texture { texture_id, size: tex_data.size, })
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

pub fn create_texture() -> u32 {
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
