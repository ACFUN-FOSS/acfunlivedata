use crate::config::{User, CONFIG, TOKEN_LENGTH};
use axum::{
    body::{box_body, Body, BoxBody},
    http::{Request, Response, StatusCode},
};
use tower_http::auth::require_authorization::AuthorizeRequest;

const AUTH_HEADER: &str = "token";

#[derive(Clone, Copy, Debug)]
pub struct Token;

impl AuthorizeRequest for Token {
    type Output = User;

    type ResponseBody = BoxBody;

    fn authorize<B>(&mut self, request: &Request<B>) -> Option<Self::Output> {
        let mut headers = request.headers().get_all(AUTH_HEADER).iter();
        let token = match headers.next() {
            Some(v) => v.to_str().ok()?,
            None => return None,
        };
        // 长度不符合
        if token.len() != TOKEN_LENGTH {
            return None;
        }
        // header数量大于一
        if headers.next().is_some() {
            return None;
        }

        let config =
            futures::executor::block_on(CONFIG.get().expect("failed to get CONFIG").read());

        config.get(token)
    }

    #[inline]
    fn unauthorized_response<B>(&mut self, request: &Request<B>) -> Response<Self::ResponseBody> {
        log::warn!(
            "failed to authorize, token: {:?}",
            request
                .headers()
                .get_all(AUTH_HEADER)
                .iter()
                .map(|v| v.to_str())
                .collect::<Vec<_>>()
        );
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(box_body(Body::empty()))
            .expect("failed to build Response")
    }

    #[inline]
    fn on_authorized<B>(&mut self, request: &mut Request<B>, output: Self::Output) {
        let _ = request.extensions_mut().insert(output);
    }
}
