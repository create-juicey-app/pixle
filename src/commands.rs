#[derive(Clone, Copy, Debug)]
pub enum PaintCommand {
    // Added 'a' (alpha)
    DrawPixel {
        x: u32,
        y: u32,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
}
