use crate::{config::User, model::QueryRoot};
use async_graphql::{
    extensions::Logger,
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptyMutation, EmptySubscription, Request, Response, Schema,
};
use axum::prelude::*;

pub type LiveSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

#[inline]
pub fn schema() -> LiveSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .extension(Logger)
        .finish()
}

#[inline]
pub async fn graphql_playground() -> impl response::IntoResponse {
    response::Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

#[inline]
pub async fn graphql_handler(
    schema: extract::Extension<LiveSchema>,
    user: extract::Extension<User>,
    req: extract::Json<Request>,
) -> response::Json<Response> {
    let req = req.0.data(user.0);
    schema.execute(req).await.into()
}
