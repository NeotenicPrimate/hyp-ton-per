use crate::hello_world::greeter_server::Greeter;
use crate::hello_world::{HelloReply, HelloRequest};

#[derive(Default, Debug)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(&self, request: tonic::Request<HelloRequest>) -> Result<tonic::Response<HelloReply> , tonic::Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(tonic::Response::new(reply))
    }
}