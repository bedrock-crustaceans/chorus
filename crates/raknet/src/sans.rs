pub trait Sans {
    type Input;
    type Output;
    type Error;

    fn handle(&mut self, msg: Self::Input) -> Result<(), Self::Error>;

    fn poll(&mut self) -> Option<Self::Output>;
}
