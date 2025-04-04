/// Creates a boxed array of the given element.
pub fn boxed_array<T, const LEN: usize>(value: T) -> Box<[T; LEN]>
where
    T: Clone,
{
    let v = vec![value; LEN];
    let boxed_slice = v.into_boxed_slice();
    Box::try_from(boxed_slice)
        .ok()
        .expect("boxed slice should have exactly LEN elements")
}
