pub mod display;
pub mod environment;
pub mod rendering;

use self::{display::*, environment::*, rendering::*};
use bitos::{bitos, integer::u2};

/// The primary opcode of a [`GpuInstruction`].
#[bitos(3)]
#[derive(Debug, PartialEq, Eq)]
pub enum GpuOpcode {
    Misc = 0,
    Polygon = 1,
    Line = 2,
    Rectangle = 3,
    VramToVramBlit = 4,
    CpuToVramBlit = 5,
    VramToCpuBlit = 6,
    Environment = 7,
}

/// The misc opcode of a [`GpuInstruction`].
#[bitos(2)]
#[derive(Debug, PartialEq, Eq)]
pub enum MiscOpcode {
    /// Does nothing.
    NOP = 0,
    /// Clear the texture cache of the GPU.
    ClearCache = 1,
    /// Fills an area in the frame buffer with a color. Requires 2 data packets.
    QuickRectangleFill = 2,
}

/// The environment opcode of a [`GpuInstruction`].
#[bitos(3)]
#[derive(Debug, PartialEq, Eq)]
pub enum EnvironmentOpcode {
    /// Set the drawing setings.
    DrawingSettings = 1,
    /// Setthe texture window settings.
    TexWindowSettings = 2,
    /// Set the top-left position of the drawing area.
    DrawingAreaTopLeft = 3,
    /// Set the bottom-right position of the drawing area.
    DrawingAreaBottomRight = 4,
    /// Set the offset of the drawing area.
    DrawingOffset = 5,
    // TODO: document
    MaskBit = 6,
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
    ReadGpuRegister = 0x10,
    VramSize = 0x20,
}

/// A Display instruction. Received through GP1.
#[bitos(32)]
pub struct DisplayInstruction {
    #[bits(24..30)]
    pub opcode: Option<DisplayOpcode>,

    #[bits(..)]
    pub display_enable_instr: DisplayEnableInstr,
    #[bits(..)]
    pub dma_direction_instr: DmaDirectionInstr,
    #[bits(..)]
    pub display_area_instr: DisplayAreaInstr,
    #[bits(..)]
    pub horizontal_display_range_instr: HorizontalDisplayRangeInstr,
    #[bits(..)]
    pub vertical_dispaly_range_instr: VerticalDisplayRangeInstr,
    #[bits(..)]
    pub display_mode_instr: DisplayModeInstr,
}

impl std::fmt::Debug for DisplayInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.opcode() {
            Some(opcode) => match opcode {
                DisplayOpcode::ResetGpu => write!(f, "ResetGpu"),
                DisplayOpcode::ResetCommandBuffer => write!(f, "ResetCommandBuffer"),
                DisplayOpcode::AcknowledgeGpuInterrupt => write!(f, "AcknowledgeGpuInterrupt"),
                DisplayOpcode::DisplayEnabled => self.display_enable_instr().fmt(f),
                DisplayOpcode::DmaDirection => self.dma_direction_instr().fmt(f),
                DisplayOpcode::DisplayArea => self.display_area_instr().fmt(f),
                DisplayOpcode::HorizontalDisplayRange => {
                    self.horizontal_display_range_instr().fmt(f)
                }
                DisplayOpcode::VerticalDisplayRange => self.vertical_dispaly_range_instr().fmt(f),
                DisplayOpcode::DisplayMode => self.display_mode_instr().fmt(f),
                DisplayOpcode::ReadGpuRegister => write!(f, "ReadGpuRegister"),
                DisplayOpcode::VramSize => write!(f, "VramSize"),
            },
            None => write!(f, "unknown opcode"),
        }
    }
}

/// A GPU instruction. Received through GP0.
#[bitos(32)]
pub struct GpuInstruction {
    #[bits(29..32)]
    pub opcode: GpuOpcode,
    #[bits(24..26)]
    pub misc_opcode: Option<MiscOpcode>,
    #[bits(24..26)]
    pub misc_opcode_raw: u2,
    #[bits(24..27)]
    pub environment_opcode: Option<EnvironmentOpcode>,

    #[bits(..)]
    pub polygon_instr: PolygonInstr,
    #[bits(..)]
    pub line_instr: LineInstr,
    #[bits(..)]
    pub rectangle_instr: RectangleInstr,

    #[bits(..)]
    pub drawing_settings_instr: DrawingSettingsInstr,
    #[bits(..)]
    pub texture_window_settings_instr: TextureWindowSettingsInstr,
    #[bits(..)]
    pub drawing_area_corner_instr: DrawingAreaCornerInstr,
    #[bits(..)]
    pub drawing_offset_instr: DrawingOffsetInstr,
    #[bits(..)]
    pub mask_bit_settings_instr: MaskBitSettingsInstr,
}

impl std::fmt::Debug for GpuInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.opcode() {
            GpuOpcode::Misc => match self.misc_opcode() {
                Some(misc_opcode) => match misc_opcode {
                    MiscOpcode::NOP => write!(f, "NOP"),
                    MiscOpcode::ClearCache => write!(f, "ClearCache"),
                    MiscOpcode::QuickRectangleFill => write!(f, "QuickRectangleFill"),
                },
                None => write!(f, "unknown misc opcode"),
            },
            GpuOpcode::Polygon => self.polygon_instr().fmt(f),
            GpuOpcode::Line => self.line_instr().fmt(f),
            GpuOpcode::Rectangle => self.rectangle_instr().fmt(f),
            GpuOpcode::VramToVramBlit => write!(f, "VramToVramBlit"),
            GpuOpcode::CpuToVramBlit => write!(f, "CpuToVramBlit"),
            GpuOpcode::VramToCpuBlit => write!(f, "VramToCpuBlit"),
            GpuOpcode::Environment => match self.environment_opcode() {
                Some(env_opcode) => match env_opcode {
                    EnvironmentOpcode::DrawingSettings => self.drawing_settings_instr().fmt(f),
                    EnvironmentOpcode::TexWindowSettings => {
                        self.texture_window_settings_instr().fmt(f)
                    }
                    EnvironmentOpcode::DrawingAreaTopLeft => {
                        self.drawing_area_corner_instr().fmt(f)
                    }
                    EnvironmentOpcode::DrawingAreaBottomRight => {
                        self.drawing_area_corner_instr().fmt(f)
                    }
                    EnvironmentOpcode::DrawingOffset => self.drawing_offset_instr().fmt(f),
                    EnvironmentOpcode::MaskBit => self.mask_bit_settings_instr().fmt(f),
                },
                None => write!(f, "unknown environment opcode"),
            },
        }
    }
}
