use snui::snui::*;
use snui::wayland::*;
use snui::widgets::*;
use wayland_client::protocol::{
    wl_surface::WlSurface,
};
use std::path::Path;
use wayland_client::Main;
use smithay_client_toolkit::shm::AutoMemPool;
use std::io::{Write, BufWriter};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::{
    zwlr_layer_surface_v1,
    zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
};

#[derive(Clone, Debug)]
pub enum Paper {
    Color(u32),
    Tiled(String),
    Border(String, u32, u32),
    Image(String),
    None
}

impl Paper {
    pub fn is_some(&self) -> bool {
        match self {
            Paper::None => false,
            _ => true
        }
    }
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
        layer_surface.set_size(width as u32, height as u32);
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
        match paper {
            Paper::Color(color) => {
                let pxcount = buffer.size()/4;
                let mut writer = BufWriter::new(buffer.get_mut_buf());
                for _ in 0..pxcount {
                    writer.write_all(&color.to_ne_bytes()).unwrap();
                }
                writer.flush().unwrap();
            }
            Paper::Image(path) => {
                let path = Path::new(&path);
                let mut image = Image::new(path).unwrap();
                image.resize(width, height);
                buffer.composite(&to_surface(&image), 0, 0);
            }
            Paper::Border(path, gap, color) => {
                let path = Path::new(&path);
                let bg = image_with_border(path, width , height, *gap, *color);
                buffer.composite(&to_surface(&bg), 0, 0);
            }
            Paper::Tiled(path) => {
                let path = Path::new(&path);
                let bg = tile(path, self.width as u32, self.height as u32);
                buffer.composite(&bg, 0, 0);
            }
            Paper::None => {}
        }
        buffer.attach(&self.surface, 0, 0);
    }
    pub fn dispatch_surface(mut self, paper: Paper) {
        self.layer_surface.clone().quick_assign(move |layer_surface, event, _| {
            match event {
                zwlr_layer_surface_v1::Event::Configure {
                    serial,
                    width,
                    height,
                } => {
                    layer_surface.ack_configure(serial);
                    layer_surface.set_size(width, height);
                    self.mempool.resize((width * height) as usize).unwrap();

                    // The client should use commit to notify itself
                    // that it has been configured
                    // The client is also responsible for damage
                    self.draw(&paper, width, height);
                    // self.mempool.fill();
                    self.surface.commit();
                }
                zwlr_layer_surface_v1::Event::Closed => {
                    self.destroy();
                }
                _ => {}
            }
        });
    }
}

pub fn tile(path: &Path, width: u32, height: u32) -> Surface {
    let mut y = 0;
    let image = Image::new(path);
    let mut surface = Surface::empty(width, height);
    let img_width = image.as_ref().unwrap().get_width();
    let img_height = image.as_ref().unwrap().get_height();
    while y < height {
        let mut x = 0;
        while x < width {
            image.as_ref().unwrap().draw(&mut surface, x, y);
            x += img_width;
        }
        y += img_height;
    }
    surface
}

fn image_with_border(path: &Path, width: u32, height: u32, border_size: u32, color: u32) -> Node {
    let gap = border_size * 2;
	let mut image = Image::new(path).unwrap();
    image.resize(width - gap, height - gap);
    border(image, border_size, Content::Pixel(color))
}
