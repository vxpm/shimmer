use super::cmd::{
    display::{DisplayAreaCmd, DisplayModeCmd},
    environment::DrawingSettingsCmd,
    rendering::ShadingMode,
};
use bitos::integer::{i11, u10};

#[derive(Debug, Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: i11,
    pub y: i11,
    pub u: u8,
    pub v: u8,
    pub color: Rgb,
}

#[derive(Debug, Clone)]
pub struct UntexturedTriangle {
    pub vertices: [Vertex; 3],
    pub shading_mode: ShadingMode,
}

#[derive(Debug, Clone)]
pub struct CopyToVram {
    pub x: u10,
    pub y: u10,
    pub data: Vec<u8>,
}

/// A renderer action.
///
/// This is almost like a GPU command, and some variants are really just wrappers over a command,
/// but this type always contains all the data necessary to execute.
#[derive(Debug, Clone)]
pub enum Action {
    // Configuration
    Reset,
    DrawSettings(DrawingSettingsCmd),
    DisplayMode(DisplayModeCmd),
    DisplayArea(DisplayAreaCmd),

    // Copy data
    CopyToVram(CopyToVram),

    // Draw stuff
    DrawUntexturedTriangle(UntexturedTriangle),
}
