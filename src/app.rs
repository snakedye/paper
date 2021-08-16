use snui::*;
use std::io;
use std::fs;
use std::path::Path;
use rand::thread_rng;
use snui::wayland::*;
use snui::widgets::*;
use rand::seq::IteratorRandom;
use std::io::{Write, BufWriter};

#[derive(Clone, Debug)]
pub struct Paper {
    style: Style,
    pub border: Option<(u32, u32)>,
    pub output: Option<String>
}

impl Paper {
    pub fn is_some(&self) -> bool {
        match self.style {
            Style::None => false,
            _ => true
        }
    }
    pub fn default() -> Self {
        Paper {
            style: Style::None,
            border: None,
            output: None
        }
    }
    pub fn style(&mut self, style: Style) {
        self.style = style
    }
    pub fn border(&mut self, gap: u32, color: u32) {
        self.border = Some((gap, color));
   }
}

#[derive(Clone, Debug)]
pub enum Style {
    Color(u32),
    Tiled(String),
    Image(String),
    Directory(String),
    None,
}

pub fn draw(buffer: &mut Buffer, paper: &Paper,  width: u32, height: u32) {
    match &paper.style {
        Style::Color(color) => {
            let pxcount = buffer.size()/4;
            let mut writer = BufWriter::new(buffer.get_mut_buf());
            for _ in 0..pxcount {
                writer.write_all(&color.to_ne_bytes()).unwrap();
            }
            writer.flush().unwrap();
        }
        Style::Image(path) => {
            let path = Path::new(&path);
            let image = Image::new_with_size(path, width as u32, height as u32).unwrap();
            image.draw(buffer.get_mut_buf(), width as u32, 0, 0);
        }
        Style::Tiled(path) => {
            let mut y = 0;
            let path = Path::new(&path);
            let image = Image::new(path);
            let img_width = image.as_ref().unwrap().get_width();
            let img_height = image.as_ref().unwrap().get_height();
            while y < height {
                let mut x = 0;
                while x < width {
                    image.as_ref().unwrap().draw(buffer.get_mut_buf(), width, x, y);
                    x += img_width;
                }
                y += img_height;
            }
        }
        Style::Directory(path) => {
            let dir = Path::new(&path);
            if dir.is_dir() {
                match random_image(dir, width as u32, height as u32) {
                    Ok(image) =>  image.draw(buffer.get_mut_buf(), width as u32, 0, 0),
                    Err(e) => eprintln!("{}", e)
                }
            } else {
                eprintln!("\"{}\" is not a directory", path);
                std::process::exit(1);
            }
        }
        _ => {}
    }
    if let Some((gap, color)) = paper.border {
        let border_hor = Rectangle::new(width, gap, color);
        let border_ver = Rectangle::new(gap, height, color);
        border_ver.draw(buffer.get_mut_buf(), width, 0, 0);
        border_hor.draw(buffer.get_mut_buf(), width, 0, 0);
        border_hor.draw(buffer.get_mut_buf(), width, 0, height-gap);
        border_ver.draw(buffer.get_mut_buf(), width, width-gap, 0);
    }
}

fn random_image(dir: &Path, width: u32, height: u32) -> io::Result<Image> {
    if dir.is_dir() {
        let mut rng = thread_rng();
        if let Some(entry) = fs::read_dir(dir)?.choose(&mut rng) {
            let path = entry?.path();
            if let Some(filename) = path.file_name() {
                let filename = filename.to_str().unwrap();
                if filename.ends_with(".png")
                || filename.ends_with(".jpeg")
                || filename.ends_with(".jpg") {
                    return Ok(Image::new_with_size(Path::new(path.to_str().unwrap()), width, height).unwrap())
                } else if path.is_dir() {
                    let dir = format!("{:?}", path);
                    return random_image(Path::new(dir.trim_matches('"')), width, height)
                }
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty directory"))
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "invalid file type"))
}
