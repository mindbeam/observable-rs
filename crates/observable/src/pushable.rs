pub trait Pushable {
    type Value;
    fn push(&mut self, value: Self::Value);
}

impl<T> Pushable for Vec<T> {
    type Value = T;
    fn push(&mut self, value: Self::Value) {
        self.push(value)
    }
}
