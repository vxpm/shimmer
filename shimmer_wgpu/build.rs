use wgsl_composer::{build, Config};

fn main() {
    let config = Config {
        input_path: "shaders/src".into(),
        output_path: "shaders/built".into(),
    };

    build(config);
}
