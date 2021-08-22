use acfunliveapi::client::{DefaultApiClient, DefaultApiClientBuilder};
use anyhow::Result;
use cached::proc_macro::cached;
use once_cell::sync::Lazy;

static API_CLIENT_BUILDER: Lazy<DefaultApiClientBuilder> = Lazy::new(|| {
    DefaultApiClientBuilder::default_client().expect("failed to construct ApiClientBuilder")
});

#[cached(size = 1, time = 3600, result = true)]
#[inline]
pub async fn build_client() -> Result<DefaultApiClient> {
    Ok(API_CLIENT_BUILDER.clone().build().await?)
}
