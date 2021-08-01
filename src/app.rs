use snui::*;
use snui::wayland::*;
use snui::widgets::*;
use wayland_client::protocol::{
    wl_surface::WlSurface,
};
use std::io;
use std::fs;
use rand::thread_rng;
use rand::seq::IteratorRandom;
use std::path::Path;
use wayland_client::Main;
use smithay_client_toolkit::shm::AutoMemPool;
use std::io::{Write, BufWriter};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

#[derive(Clone, Debug)]
pub struct Paper {
    style: Style,
    border: Option<(u32, u32)>,
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
            border: None
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

pub struct Snape {
    width: i32,
    height: i32,
    pub mempool: AutoMemPool,
    pub surface: Main<WlSurface>,
    pub layer_surface: Main<ZwlrLayerSurfaceV1>,
}

impl Snape {
    pub fn new(
        width: i32,
        height: i32,
        surface: Main<WlSurface>,
        layer_surface: Main<ZwlrLayerSurfaceV1>,
        mempool: AutoMemPool
    ) -> Snape {
        layer_surface.set_size(0, 0);
        surface.commit();
        Snape {
            width,
            height,
            mempool,
            layer_surface,
            surface
        }
    }
    fn destroy(&self) {
        self.surface.destroy();
        self.layer_surface.destroy();
    }
    fn draw(&mut self, paper: &Paper, width: u32, height: u32) {
        let mut buffer = Buffer::new(
            self.width,
            self.height,
            (4 * self.width) as i32,
            &mut self.mempool,
        );
        self.layer_surface.set_exclusive_zone(-1);
        match &paper.style {
            Style::Color(color) => {
                let pxcount = buffer.size()/4;
                self.layer_surface.set_exclusive_zone(-1);
                let mut writer = BufWriter::new(buffer.get_mut_buf());
                for _ in 0..pxcount {
                    writer.write_all(&color.to_ne_bytes()).unwrap();
                }
                writer.flush().unwrap();
            }
            Style::Image(path) => {
                let path = Path::new(&path);
                let image = Image::new_with_size(path, self.width as u32, self.height as u32).unwrap();
                image.draw(buffer.get_mut_buf(), self.width as u32, 0, 0);
            }
            Style::Tiled(path) => {
                let path = Path::new(&path);
                let bg = tile(path, self.width as u32, self.height as u32);
                buffer.composite(&bg, 0, 0);
            }
            Style::Directory(path) => {
                let dir = Path::new(&path);
                if dir.is_dir() {
                    match random_image(dir, self.width as u32, self.height as u32) {
                        Ok(image) =>  image.draw(buffer.get_mut_buf(), self.width as u32, 0, 0),
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
            self.layer_surface.set_exclusive_zone(1);
            let border_hor = Rectangle::new(width, gap, color);
            let border_ver = Rectangle::new(gap, height, color);
            border_ver.draw(buffer.get_mut_buf(), width, 0, 0);
            border_hor.draw(buffer.get_mut_buf(), width, 0, 0);
            border_hor.draw(buffer.get_mut_buf(), width, 0, height-gap);
            border_ver.draw(buffer.get_mut_buf(), width, width-gap, 0);
        }
        buffer.attach(&self.surface, 0, 0);
        self.surface.damage(
            0,
            0,
            1 << 30,
            1 << 30
        );
        self.surface.damage_buffer(
            0,
            0,
            1 << 30,
            1 << 30
        );
    }
    pub fn dispatch_surface(mut self, paper: Paper) {
        let mut ping = 0;
        self.layer_surface.clone().quick_assign(move |layer_surface, event, _| {
            match event {
                zwlr_layer_surface_v1::Event::Configure {
                    serial,
                    width,
                    height,
                } => {
                    layer_surface.ack_configure(serial);
                    self.mempool.resize((width * height) as usize).unwrap();
                    if ping != 1 {
                        self.draw(&paper, width, height);
                        self.surface.commit();
                    }
                    ping += 1;
                }
                zwlr_layer_surface_v1::Event::Closed => {
                    self.destroy();
                }
                _ => {}
            }
        });
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

pub fn tile(path: &Path, width: u32, height: u32) -> Surface {
    let mut y = 0;
    let image = Image::new(path);
    let img_width = image.as_ref().unwrap().get_width();
    let img_height = image.as_ref().unwrap().get_height();
    let surface_width = img_width * (width as f64/img_width as f64).ceil() as u32;
    let surface_height = img_height * (height as f64/img_height as f64).ceil() as u32;
    let mut surface = Surface::empty(surface_width, surface_height);
    while y < height {
        let mut x = 0;
        while x < width {
            image.as_ref().unwrap().draw(surface.get_mut_buf(), surface_width, x, y);
            x += img_width;
        }
        y += img_height;
    }
    surface
}
