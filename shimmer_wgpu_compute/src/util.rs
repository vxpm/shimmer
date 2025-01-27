use encase::{ShaderSize, ShaderType};

/// Helper type for treating a slice of elements as a shader type.
#[derive(ShaderType)]
pub struct ShaderSlice<'a, T: ShaderType + ShaderSize + 'static> {
    #[size(runtime)]
    pub value: &'a [T],
}

impl<'a, T: ShaderType + ShaderSize> ShaderSlice<'a, T> {
    pub fn new(value: &'a [T]) -> Self {
        Self { value }
    }
}
