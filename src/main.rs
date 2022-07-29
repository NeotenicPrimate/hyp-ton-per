mod grpc;
mod graphql;
mod http_switch;

use std::sync::Arc;

use tonic::{transport::Server as TonicServer, Response};
use futures::future::{self, Either, TryFutureExt};

use graphql::{Query, Context};

use hello_world::greeter_server::{GreeterServer};

use crate::grpc::MyGreeter;

pub mod hello_world {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

pub mod helloworld { tonic::include_proto!("helloworld"); }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let db = Arc::new(Context {});
    
    let root_node = Arc::new(
        juniper::RootNode::new(
            Query,
            juniper::EmptyMutation:: <Context> ::new(),
            juniper::EmptySubscription:: <Context> ::new(),
        )
    );
    
    hyper::Server::bind(&addr)
        .serve(hyper::service::make_service_fn(move |_| {

            let greeter = MyGreeter::default();

            let mut tonic = TonicServer::builder()
                .add_service(GreeterServer::new(greeter))
                .into_service();

            type Fut = Box<dyn futures::Future<Output = http::Response<hyper::Body>>>;

            type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

            future::ok::<_, std::convert::Infallible>(tower::service_fn(
                move |req: hyper::Request<hyper::Body>| match req.version() {
                    http::Version::HTTP_11 | http::Version::HTTP_10 => Either::Left(
                        match (req.method(), req.uri().path()) {
                            (&http::Method::GET, "/") => {
                                Box::new(juniper_hyper::graphiql("/graphql", None)) as Fut
                            },
                            (&http::Method::GET, "/graphql") | (&http::Method::POST, "/graphql") => {
                                let ctx = db.clone();
                                Box::new(juniper_hyper::graphql(root_node, ctx, req)) as Fut
                            },
                            _ => {
                                let mut response = Response::new(hyper::Body::empty());
                                Box::new(response) as Fut
                            },
                        }
                    ),
                    http::Version::HTTP_2 => Either::Right(
                        tonic
                            .call(req)
                            .map_ok(|res| res.map(http_switch::EitherBody::Right))
                            .map_err(Error::from),
                    ),
                    _ => unimplemented!(),
                },
            ))
        }))
        .await?;

    Ok(())
}

