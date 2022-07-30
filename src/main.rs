mod graphql;
mod grpc;
mod http_switch;

use std::sync::Arc;

use futures::future::{self, Either, TryFutureExt};
use tonic::{transport::Server as TonicServer, Response};
use tower::Service;

use graphql::{Context, Query};

use hello_world::greeter_server::GreeterServer;

use crate::{grpc::MyGreeter, http_switch::EitherBody};

pub mod hello_world {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let db = Arc::new(Context {});

    let root_node = Arc::new(juniper::RootNode::new(
        Query,
        juniper::EmptyMutation::<Context>::new(),
        juniper::EmptySubscription::<Context>::new(),
    ));

    hyper::Server::bind(&addr)
        .serve(hyper::service::make_service_fn(move |_| {
            let greeter = MyGreeter::default();

            let mut tonic = TonicServer::builder()
                .add_service(GreeterServer::new(greeter))
                .into_service();

            type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
            type Fut = std::pin::Pin<
                Box<
                    dyn futures::Future<
                            Output = Result<
                                http::Response<EitherBody<hyper::Body, tonic::body::BoxBody>>,
                                Error,
                            >,
                        > + Send,
                >,
            >;

            let db = db.clone();
            let root_node = root_node.clone();
            future::ok::<_, std::convert::Infallible>(tower::service_fn(
                move |req: hyper::Request<hyper::Body>| match req.version() {
                    http::Version::HTTP_11 | http::Version::HTTP_10 => {
                        match (req.method(), req.uri().path()) {
                            (&http::Method::GET, "/") => {
                                // Box::new(juniper_hyper::graphiql("/graphql", None)) as Fut
                                Box::pin(async move {
                                    let res = juniper_hyper::graphiql("/graphql", None)
                                        .await
                                        .map(EitherBody::Left);

                                    Ok(res)
                                }) as Fut
                            }
                            (&http::Method::GET, "/graphql")
                            | (&http::Method::POST, "/graphql") => {
                                let ctx = db.clone();
                                let root_node = root_node.clone();
                                // Box::new(juniper_hyper::graphql(root_node, ctx, req)) as Fut;
                                Box::pin(async move {
                                    let res = juniper_hyper::graphql(root_node, ctx, req)
                                        .await
                                        .map(EitherBody::Left);
                                    Ok(res)
                                }) as Fut
                            }
                            _ => {
                                let mut response =
                                    http::Response::new(EitherBody::Left(hyper::Body::empty()));
                                Box::pin(async move { Ok(response) })
                            }
                        }
                    }
                    http::Version::HTTP_2 => Box::pin({
                        let mut tonic = tonic.clone();
                        async move {
                            tonic
                                .call(req)
                                .map_ok(|res| res.map(http_switch::EitherBody::Right))
                                .map_err(Error::from)
                                .await
                        }
                    }),
                    _ => unimplemented!(),
                },
            ))
        }))
        .await?;

    Ok(())
}
