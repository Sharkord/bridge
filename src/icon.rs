pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

// the Sharkord icon from the web client, shared by the tray, the window and
// the desktop entry
pub fn load() -> Icon {
    let mut reader = png::Decoder::new(std::io::Cursor::new(ICON_PNG))
        .read_info()
        .expect("the embedded icon is a valid png");
    let mut rgba = vec![0; reader.output_buffer_size().expect("the icon has a size")];
    let info = reader
        .next_frame(&mut rgba)
        .expect("the embedded icon decodes");

    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "the icon must be rgba"
    );
    assert_eq!(
        info.bit_depth,
        png::BitDepth::Eight,
        "the icon must be 8 bit"
    );

    rgba.truncate(info.buffer_size());

    Icon {
        width: info.width,
        height: info.height,
        rgba,
    }
}

pub fn png_bytes() -> &'static [u8] {
    ICON_PNG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_to_a_square_rgba_image_that_is_not_blank() {
        let icon = load();

        assert_eq!(icon.width, icon.height);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
    }
}
