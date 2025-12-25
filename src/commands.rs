#[derive(Clone, Copy, Debug)]
pub enum PaintCommand {
    DrawPixel { x: u32, y: u32, r: u8, g: u8, b: u8 },
}
