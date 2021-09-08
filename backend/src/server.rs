use crate::{auth::Token, config::User, download::Download, model::QueryRoot};
use async_graphql::{
    extensions::Logger,
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request as GraphqlRequest, Response as GraphqlResponse,
    Schema,
};
use axum::{
    extract, handler::get, handler::Handler, http::StatusCode, response, service,
    AddExtensionLayer, Router,
};
use std::{convert::Infallible, time::Duration};
use tower::{
    limit::concurrency::ConcurrencyLimitLayer,
    load_shed::{error::Overloaded, LoadShedLayer},
    timeout::{error::Elapsed, TimeoutLayer},
    BoxError, Service, ServiceExt,
};
use tower_http::{
    auth::require_authorization::RequireAuthorizationLayer, compression::CompressionLayer,
    services::fs::ServeFile,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const CONCURRENCY_LIMIT: usize = 50;
const HTTP2_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);
const HTTP2_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);
const TCP_KEEPALIVE: Duration = Duration::from_secs(30);

type LiveSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub async fn graphql_server() {
    let schema = schema();
    //println!("{}", schema.sdl());

    let app = Router::new()
        .route(
            "/",
            get(graphql_playground).post(
                graphql_handler
                    .layer(RequireAuthorizationLayer::custom(Token))
                    .layer(AddExtensionLayer::new(schema)),
            ),
        )
        .or(Router::new()
            .route(
                "/download",
                service::get(Download.then(|r| async {
                    match r {
                        Ok((p, h)) => {
                            let mut resp = ServeFile::new(p).call(()).await?;
                            resp.headers_mut().extend(h);
                            Ok(resp)
                        }
                        Err(e) => {
                            log::error!("failed to prepare downloading database: {}", e);
                            Err(e)
                        }
                    }
                }))
                .handle_error(|_| Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)),
            )
            .layer(RequireAuthorizationLayer::custom(Token)))
        .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
        .handle_error(|e: BoxError| {
            log::warn!("server receiving a request from client is timeout");
            if e.is::<Elapsed>() {
                Ok::<_, Infallible>(StatusCode::REQUEST_TIMEOUT)
            } else {
                Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
            }
        })
        .layer(LoadShedLayer::new())
        .handle_error(|e: BoxError| {
            log::warn!("server is overloaded");
            if e.is::<Overloaded>() {
                Ok::<_, Infallible>(StatusCode::TOO_MANY_REQUESTS)
            } else {
                Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
            }
        })
        .layer(ConcurrencyLimitLayer::new(CONCURRENCY_LIMIT))
        .layer(CompressionLayer::new().gzip(true).no_deflate().no_br());

    hyper::Server::bind(&"0.0.0.0:3456".parse().expect("failed to parse ip address"))
        .http1_keepalive(true)
        .http2_keep_alive_interval(HTTP2_KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_timeout(HTTP2_KEEP_ALIVE_TIMEOUT)
        .tcp_keepalive(Some(TCP_KEEPALIVE))
        .serve(app.into_make_service())
        .await
        .expect("failed to serve server");
}

#[inline]
fn schema() -> LiveSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .extension(Logger)
        .finish()
}

#[inline]
async fn graphql_playground() -> impl response::IntoResponse {
    response::Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[inline]
async fn graphql_handler(
    schema: extract::Extension<LiveSchema>,
    user: extract::Extension<User>,
    req: extract::Json<GraphqlRequest>,
) -> response::Json<GraphqlResponse> {
    let req = req.0.data(user.0);
    schema.execute(req).await.into()
}
