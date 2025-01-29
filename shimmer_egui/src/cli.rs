use clap::{Args, Parser};
use std::path::PathBuf;

fn clap_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Color, Style};
    clap::builder::Styles::styled()
        .header(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::Green))),
        )
        .usage(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::Green))),
        )
        .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightMagenta))))
        .invalid(Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightRed))))
        .valid(
            Style::new()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightGreen))),
        )
        .error(
            Style::new()
                .bold()
                .fg_color(Some(Color::Ansi(AnsiColor::Red))),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}

#[derive(Debug, Args)]
pub struct CliArgs {
    /// Path to the BIOS to use.
    #[arg(short, long)]
    pub bios: Option<PathBuf>,
    /// Path to the ROM.
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    /// Path to the EXE to sideload.
    #[arg(short, long)]
    pub sideload_exe: Option<PathBuf>,
}

/// shimmer psx emulator
#[derive(Debug, Parser)]
#[command(name = "shimmer")]
#[command(styles = clap_styles())]
pub struct Cli {
    #[command(flatten)]
    pub args: CliArgs,
}
