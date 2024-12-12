use std::future::Future;

pub trait Component {
    type Request: Request;
    type Response: Response;
    type Api: Api;

    fn api(&self) -> Self::Api;
    fn run(&self) -> impl Future<Output = ()>;
}

pub trait Api {
    type Request;
    type Response;
    type Handler: Handler;
}

pub trait Request {
    type Response: Response;
}

pub trait Response {}

pub trait Handler {
    type Response: Response;
}
