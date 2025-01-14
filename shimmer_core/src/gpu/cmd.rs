pub mod display;
pub mod environment;
pub mod rendering;

use self::{display::*, environment::*, rendering::*};
use bitos::bitos;

/// The primary opcode of a [`RenderingCommand`].
#[bitos(3)]
#[derive(Debug, PartialEq, Eq)]
pub enum RenderingOpcode {
    Misc = 0x0,
    Polygon = 0x1,
    Line = 0x2,
    Rectangle = 0x3,
    VramToVramBlit = 0x4,
    CpuToVramBlit = 0x5,
    VramToCpuBlit = 0x6,
    Environment = 0x7,
}

/// The misc opcode of a [`RenderingCommand`].
#[bitos(5)]
#[derive(Debug, PartialEq, Eq)]
pub enum MiscOpcode {
    /// Does nothing.
    NOP = 0x00,
    /// Clear the texture cache of the GPU.
    ClearCache = 0x01,
    /// Fills an area in the frame buffer with a color.
    QuickRectangleFill = 0x02,
    InterruptRequest = 0x1F,
}

/// The environment opcode of a [`RenderingCommand`].
#[bitos(3)]
#[derive(Debug, PartialEq, Eq)]
pub enum EnvironmentOpcode {
    /// Set the drawing setings.
    DrawingSettings = 0x1,
    /// Setthe texture window settings.
    TexWindowSettings = 0x2,
    /// Set the top-left position of the drawing area.
    DrawingAreaTopLeft = 0x3,
    /// Set the bottom-right position of the drawing area.
    DrawingAreaBottomRight = 0x4,
    /// Set the offset of the drawing area.
    DrawingOffset = 0x5,
    MaskSettings = 0x6,
}

#[bitos(6)]
#[derive(Debug, PartialEq, Eq)]
pub enum DisplayOpcode {
    ResetGpu = 0x0,
    ResetCommandBuffer = 0x1,
    AcknowledgeGpuInterrupt = 0x2,
    DisplayEnabled = 0x3,
    DmaDirection = 0x4,
    DisplayArea = 0x5,
    HorizontalDisplayRange = 0x6,
    VerticalDisplayRange = 0x7,
    DisplayMode = 0x8,
    VramSizeV2 = 0x09,
    ReadGpuRegister = 0x10,
    VramSizeV1 = 0x20,
}

/// A display command. Received through GP1.
#[bitos(32)]
#[derive(Clone, Copy)]
pub struct DisplayCommand {
    #[bits(24..30)]
    pub opcode: Option<DisplayOpcode>,

    #[bits(..)]
    pub display_enable_cmd: DisplayEnableCmd,
    #[bits(..)]
    pub dma_direction_cmd: DmaDirectionCmd,
    #[bits(..)]
    pub display_area_cmd: DisplayAreaCmd,
    #[bits(..)]
    pub horizontal_display_range_cmd: HorizontalDisplayRangeCmd,
    #[bits(..)]
    pub vertical_dispaly_range_cmd: VerticalDisplayRangeCmd,
    #[bits(..)]
    pub display_mode_cmd: DisplayModeCmd,
    #[bits(..)]
    pub vram_size_cmd: VramSizeCmd,
}

impl std::fmt::Debug for DisplayCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.opcode() {
            Some(opcode) => match opcode {
                DisplayOpcode::ResetGpu => write!(f, "ResetGpu"),
                DisplayOpcode::ResetCommandBuffer => write!(f, "ResetCommandBuffer"),
                DisplayOpcode::AcknowledgeGpuInterrupt => write!(f, "AcknowledgeGpuInterrupt"),
                DisplayOpcode::DisplayEnabled => self.display_enable_cmd().fmt(f),
                DisplayOpcode::DmaDirection => self.dma_direction_cmd().fmt(f),
                DisplayOpcode::DisplayArea => self.display_area_cmd().fmt(f),
                DisplayOpcode::HorizontalDisplayRange => self.horizontal_display_range_cmd().fmt(f),
                DisplayOpcode::VerticalDisplayRange => self.vertical_dispaly_range_cmd().fmt(f),
                DisplayOpcode::DisplayMode => self.display_mode_cmd().fmt(f),
                DisplayOpcode::VramSizeV2 => self.vram_size_cmd().fmt(f),
                DisplayOpcode::ReadGpuRegister => write!(f, "ReadGpuRegister"),
                DisplayOpcode::VramSizeV1 => write!(f, "VramSizeV1"),
            },
            None => write!(f, "unknown opcode"),
        }
    }
}

/// A rendering command. Received through GP0.
#[bitos(32)]
#[derive(Clone, Copy)]
pub struct RenderingCommand {
    #[bits(29..32)]
    pub opcode: RenderingOpcode,
    #[bits(24..29)]
    pub misc_opcode: Option<MiscOpcode>,
    #[bits(24..27)]
    pub environment_opcode: Option<EnvironmentOpcode>,

    #[bits(..)]
    pub polygon_cmd: PolygonCmd,
    #[bits(..)]
    pub line_cmd: LineCmd,
    #[bits(..)]
    pub rectangle_cmd: RectangleCmd,

    #[bits(..)]
    pub drawing_settings_cmd: DrawingSettingsCmd,
    #[bits(..)]
    pub texture_window_settings_cmd: TextureWindowSettingsCmd,
    #[bits(..)]
    pub drawing_area_corner_cmd: DrawingAreaCornerCmd,
    #[bits(..)]
    pub drawing_offset_cmd: DrawingOffsetCmd,
    #[bits(..)]
    pub mask_settings_cmd: MaskSettingsCmd,
}

impl std::fmt::Debug for RenderingCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.opcode() {
            RenderingOpcode::Misc => match self.misc_opcode() {
                Some(misc_opcode) => match misc_opcode {
                    MiscOpcode::NOP => write!(f, "NOP"),
                    MiscOpcode::ClearCache => write!(f, "ClearCache"),
                    MiscOpcode::QuickRectangleFill => write!(f, "QuickRectangleFill"),
                    MiscOpcode::InterruptRequest => write!(f, "InterruptRequest"),
                },
                None => write!(f, "unknown misc opcode"),
            },
            RenderingOpcode::Polygon => self.polygon_cmd().fmt(f),
            RenderingOpcode::Line => self.line_cmd().fmt(f),
            RenderingOpcode::Rectangle => self.rectangle_cmd().fmt(f),
            RenderingOpcode::VramToVramBlit => write!(f, "VramToVramBlit"),
            RenderingOpcode::CpuToVramBlit => write!(f, "CpuToVramBlit"),
            RenderingOpcode::VramToCpuBlit => write!(f, "VramToCpuBlit"),
            RenderingOpcode::Environment => match self.environment_opcode() {
                Some(env_opcode) => match env_opcode {
                    EnvironmentOpcode::DrawingSettings => self.drawing_settings_cmd().fmt(f),
                    EnvironmentOpcode::TexWindowSettings => {
                        self.texture_window_settings_cmd().fmt(f)
                    }
                    EnvironmentOpcode::DrawingAreaTopLeft => self.drawing_area_corner_cmd().fmt(f),
                    EnvironmentOpcode::DrawingAreaBottomRight => {
                        self.drawing_area_corner_cmd().fmt(f)
                    }
                    EnvironmentOpcode::DrawingOffset => self.drawing_offset_cmd().fmt(f),
                    EnvironmentOpcode::MaskSettings => self.mask_settings_cmd().fmt(f),
                },
                None => write!(f, "unknown environment opcode"),
            },
        }
    }
}

impl RenderingCommand {
    /// How many arguments this command requires before execution can start.
    pub fn args(&self) -> usize {
        match self.opcode() {
            RenderingOpcode::Misc => match self.misc_opcode() {
                Some(MiscOpcode::QuickRectangleFill) => 2,
                _ => 0,
            },
            RenderingOpcode::Polygon => {
                let cmd = self.polygon_cmd();
                let vertices = match cmd.polygon_mode() {
                    PolygonMode::Triangle => 3,
                    PolygonMode::Rectangle => 4,
                };
                let colors = vertices
                    * match cmd.shading_mode() {
                        ShadingMode::Flat => 0,
                        ShadingMode::Gouraud => 1,
                    };
                let uvs = vertices * usize::from(cmd.textured());

                vertices + colors + uvs
            }
            RenderingOpcode::Line => {
                let cmd = self.line_cmd();
                match cmd.shading_mode() {
                    ShadingMode::Flat => 2,
                    ShadingMode::Gouraud => 4,
                }
            }
            RenderingOpcode::Rectangle => {
                let cmd = self.rectangle_cmd();
                let uv = usize::from(cmd.textured());
                let dimensions = match cmd.rectangle_mode() {
                    RectangleMode::Variable => 1,
                    _ => 0,
                };

                2 + uv + dimensions
            }
            RenderingOpcode::VramToVramBlit => 3,
            RenderingOpcode::CpuToVramBlit => 2,
            RenderingOpcode::VramToCpuBlit => 2,
            RenderingOpcode::Environment => 0,
        }
    }
}
