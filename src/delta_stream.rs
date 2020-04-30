use futures::Stream;

pub trait DeltaStream<'a, R: DeltaMessage>: Stream<Item = Result<R, ()>> {
    // fn next(&mut self) -> Next<'a, DeltaMessage>;
    fn start(&self);
    fn finish(&self);
}

pub trait DeltaMessage {
    fn read(&self);
    fn finish(&self);
}
