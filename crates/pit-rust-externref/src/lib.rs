pub fn setup<'a, 'b>(
    x: &'a mut externref::processor::Processor<'b>,
) -> &'a mut externref::processor::Processor<'b> {
    x.set_drop_fn("pit", "drop")
}
