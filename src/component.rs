pub trait Component: Sized {
    type Controller: Controller;

    fn new() -> (Self, Self::Controller);
}

pub trait Controller {}
