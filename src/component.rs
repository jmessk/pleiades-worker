
pub trait Component {
    type Request: Request;
    type Response: Response;
    type Api: Api;

    fn api(&self) -> Self::Api;
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
