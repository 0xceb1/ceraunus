
#[allow(dead_code)]
trait Processor<E> {
    type Output;
    fn process(&mut self, event: E) -> Self::Output;
}