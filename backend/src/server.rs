use crate::{auth::Token, config::User, download::Download, model::QueryRoot};
use async_graphql::{
    extensions::Logger,
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request as GraphqlRequest, Response as GraphqlResponse,
    Schema,
};
use axum::{http::StatusCode, prelude::*, service, AddExtensionLayer};
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

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONCURRENCY_LIMIT: usize = 50;
const DOWNLOAD_LIMIT: usize = 1;

type LiveSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub async fn graphql_server() {
    let schema = schema();
    //println!("{}", schema.sdl());

    let app = route(
        "/",
        get(graphql_playground).post(
            graphql_handler
                .layer(RequireAuthorizationLayer::custom(Token))
                .layer(AddExtensionLayer::new(schema)),
        ),
    )
    .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
    .handle_error(|e: BoxError| {
        if e.is::<Elapsed>() {
            Ok::<_, Infallible>(StatusCode::REQUEST_TIMEOUT)
        } else {
            Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
        }
    })
    .layer(LoadShedLayer::new())
    .handle_error(|e: BoxError| {
        if e.is::<Overloaded>() {
            Ok::<_, Infallible>(StatusCode::TOO_MANY_REQUESTS)
        } else {
            Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
        }
    })
    .layer(ConcurrencyLimitLayer::new(CONCURRENCY_LIMIT))
    .layer(CompressionLayer::new().gzip(true).no_deflate().no_br());

    axum::Server::bind(&"0.0.0.0:3000".parse().expect("failed to parse ip address"))
        .serve(app.into_make_service())
        .await
        .expect("failed to serve graphql server");
}

pub async fn download_server() {
    let app = route(
        "/download",
        service::get(Download.then(|r| async {
            match r {
                Ok((p, h)) => {
                    let mut resp = ServeFile::new(p).call(()).await?;
                    resp.headers_mut().extend(h);
                    Ok(resp)
                }
                Err(e) => Err(e),
            }
        })),
    )
    .layer(TimeoutLayer::new(REQUEST_TIMEOUT))
    .handle_error(|e: BoxError| {
        if e.is::<Elapsed>() {
            Ok::<_, Infallible>(StatusCode::REQUEST_TIMEOUT)
        } else {
            Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
        }
    })
    .layer(LoadShedLayer::new())
    .handle_error(|e: BoxError| {
        if e.is::<Overloaded>() {
            Ok::<_, Infallible>(StatusCode::TOO_MANY_REQUESTS)
        } else {
            Ok::<_, Infallible>(StatusCode::INTERNAL_SERVER_ERROR)
        }
    })
    .layer(ConcurrencyLimitLayer::new(DOWNLOAD_LIMIT))
    .layer(RequireAuthorizationLayer::custom(Token))
    .layer(CompressionLayer::new().gzip(true).no_deflate().no_br());

    axum::Server::bind(&"0.0.0.0:3001".parse().expect("failed to parse ip address"))
        .serve(app.into_make_service())
        .await
        .expect("failed to serve graphql server");
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
