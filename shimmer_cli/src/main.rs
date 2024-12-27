use shimmer_core::PSX;

fn main() {
    let bios = std::fs::read("BIOS.BIN").unwrap();
    let mut psx = PSX::with_bios(bios);

    loop {
        psx.cycle();
    }
}
