use std::{
  collections::{HashMap, BTreeMap},
  time::{SystemTime, UNIX_EPOCH},
  io::Read,
  sync::{OnceLock, Arc}
};

use base64::{engine::general_purpose, Engine};
use flate2::read::GzDecoder;
use hmac::{Hmac, Mac};
use itertools::Itertools;
use tracing::debug;
use regex::Regex;
use sha1::Sha1;
use hyper::{
  Client,
  Body,
  client::HttpConnector,
  Request,
  body,
  Response,
  http::request, header
};
use hyper_rustls::{
  HttpsConnector as rustls_HttpsConnector,
  HttpsConnectorBuilder
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use anyhow::{Result, anyhow};


type HmacSha1 = Hmac<Sha1>;
type HttpsConnector = rustls_HttpsConnector<HttpConnector>;


pub(self) static LOOKUP_USER_ID_REGEX: OnceLock<Regex> = OnceLock::new();


#[derive(Debug)]
pub struct TwitterClient {
  request_token: Option<Arc<str>>,
  request_token_secret: Option<Arc<str>>,
  access_token: Option<Arc<str>>,
  access_token_secret: Option<Arc<str>>,
  oauth: OAuthSession
}

unsafe impl Send for TwitterClient {}

impl TwitterClient {
  pub fn new(
    consumer_key: Arc<str>,
    consumer_secret: Arc<str>,
    access_token: Option<Arc<str>>,
    access_token_secret: Option<Arc<str>>
  ) -> Result<TwitterClient> {
    Ok(
      TwitterClient {
        request_token: None,
        request_token_secret: None,
        access_token: access_token.clone(),
        access_token_secret: access_token_secret.clone(),
        oauth: OAuthSession::new(
          consumer_key,
          consumer_secret,
          access_token,
          access_token_secret
        )?
      }
    )
  }

  pub async fn get_authorization_url(self: &mut Self) -> Result<Arc<str>> {
    let token: HashMap<Arc<str>, Arc<str>> = self.oauth.fetch_token(
      "https://api.twitter.com/oauth/request_token",
      BTreeMap::new()
    ).await?;

    self.request_token = token.get("oauth_token").cloned();
    self.request_token_secret = token.get("oauth_token_secret").cloned();

    Ok(
      format!(
        "https://api.twitter.com/oauth/authorize?oauth_token={oauth_token}",
        oauth_token = self.request_token
          .clone()
          .ok_or(anyhow!("Get request token failed."))?
      ).into()
    )
  }

  pub async fn get_access_token(
    self: &mut Self,
    verifier: &str
  ) -> Result<(&str, &str)> {
    (self.access_token, self.access_token_secret) = self.oauth.get_access_token(
      verifier,
      self.request_token
        .as_ref()
        .ok_or(anyhow!("Get request token failed."))?
    ).await?;

    Ok(
      (
        self.access_token
          .as_ref()
          .ok_or(anyhow!("Get access token failed."))?,
        self.access_token_secret
          .as_ref()
          .ok_or(anyhow!("Get access token secret failed."))?
      )
    )
  }

  pub async fn like(self: &mut Self, tweet_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/favorites/create.json?id={tweet_id}",
      tweet_id = tweet_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    params.insert(
      "id",
      tweet_id.into()
    );
    
    self.oauth.request(&url, params).await?;

    Ok(())
  }

  pub async fn unlike(self: &mut Self, tweet_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/favorites/destroy.json?id={tweet_id}",
      tweet_id = tweet_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    params.insert(
      "id",
      tweet_id.into()
    );
    
    self.oauth.request(&url, params).await?;

    Ok(())
  }

  pub async fn retweet(self: &mut Self, tweet_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/statuses/retweet/{tweet_id}.json",
      tweet_id = tweet_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );

    self.oauth.request(&url, params).await?;

    Ok(())
  }

  pub async fn unretweet(self: &mut Self, tweet_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/statuses/unretweet/{tweet_id}.json",
      tweet_id = tweet_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    
    self.oauth.request(&url, params).await?;

    Ok(())
  }

  pub async fn get_author_id(self: &mut Self, tweet_id: &str) -> Result<Arc<str>> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/statuses/lookup.json?id={tweet_id}&trim_user=true",
      tweet_id = tweet_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    params.insert(
      "id",
      tweet_id.into()
    );
    params.insert(
      "trim_user",
      "true".into()
    );

    Ok(
      LOOKUP_USER_ID_REGEX
        .get_or_init(
          || Regex::new(
            r#".*"user":\{"id":(?P<id>[0-9]{19}),"id_str":"[0-9]{19}"\}.*"#
          ).expect("Regex init failed.")
        )
        .replace(&self.oauth.request(&url, params).await?, "$id")
        .into()
    )
  }

  pub async fn follow(self: &mut Self, user_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/friendships/create.json?user_id={user_id}",
      user_id = user_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    params.insert(
      "user_id",
      user_id.into()
    );
    
    self.oauth.request(&url, params).await?;

    Ok(())
  }

  pub async fn unfollow(self: &mut Self, user_id: &str) -> Result<()> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/1.1/friendships/destroy.json?user_id={user_id}",
      user_id = user_id
    ).into();

    let mut params: BTreeMap<&str, Arc<str>> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token
        .clone()
        .ok_or(anyhow!("Get access token failed."))?
    );
    params.insert(
      "user_id",
      user_id.into()
    );
    
    self.oauth.request(&url, params).await?;

    Ok(())
  }
}


#[derive(Debug)]
struct OAuthSession {
  client_key: Arc<str>,
  client_secret: Arc<str>,
  resource_owner_key: Option<Arc<str>>,
  resource_owner_secret: Option<Arc<str>>,
  http_client: Client<HttpsConnector, Body>,
  hmac_sha1: HmacSha1,
  rng: StdRng
}

unsafe impl Send for OAuthSession {}

impl OAuthSession {
  fn new(
    client_key: Arc<str>,
    client_secret: Arc<str>,
    resource_owner_key: Option<Arc<str>>,
    resource_owner_secret: Option<Arc<str>>
  ) -> Result<OAuthSession> {
    let https_connector: HttpsConnector = HttpsConnectorBuilder::new()
      .with_native_roots()
      .https_only()
      .enable_http1()
      .build();

    Ok(
      OAuthSession {
        client_key,
        client_secret: client_secret.clone(),
        resource_owner_key,
        resource_owner_secret: resource_owner_secret.clone(),
        http_client: Client::builder().build(https_connector),
        hmac_sha1: HmacSha1::new_from_slice(
          format!(
            "{consumer_secret}&{token_secret}",
            consumer_secret = urlencoding::encode(&client_secret),
            token_secret = urlencoding::encode(
              &resource_owner_secret.unwrap_or("".into())
            )
          ).as_bytes()
        )?,
        rng: StdRng::from_entropy()
      }
    )
  }

  pub(self) fn update_hash_key(self: &mut Self) -> Result<()> {
    self.hmac_sha1 = HmacSha1::new_from_slice(
      format!(
        "{consumer_secret}&{token_secret}",
        consumer_secret = urlencoding::encode(&self.client_secret),
        token_secret = urlencoding::encode(
          &self.resource_owner_secret.clone().unwrap_or("".into())
        )
      ).as_bytes()
    )?;

    Ok(())
  }

  pub(self) fn apply_signture(
    self: &mut Self,
    url: &str,
    params: &mut BTreeMap<&str, Arc<str>>
  ) {
    debug!("Collected params: {:?}", params);

    let normalized_params: Arc<str> = params.iter()
      .map(
        |(k, v)|
          format!(
            "{key}={value}",
            key = urlencoding::encode(k),
            value = urlencoding::encode(v)
          ).into()
      )
      .collect::<Vec<Arc<str>>>()
      .join("&")
      .into();

    debug!("Normalized params: {:?}", normalized_params);

    let signature_base: Arc<str> = format!(
      "{method}&{url}&{params}",
      method = urlencoding::encode("POST"),
      url = urlencoding::encode(url),
      params = urlencoding::encode(&normalized_params)
    ).into();

    debug!("Signature base: {:?}", signature_base);

    self.hmac_sha1.update(signature_base.as_bytes());
    let signature: Arc<str> = general_purpose::STANDARD.encode(
      self.hmac_sha1.finalize_reset().into_bytes()
    ).into();

    debug!("Signature: {:?}", signature);

    params.insert(
      "oauth_signature",
      signature
    );
  }

  pub(self) fn apply_oauth_params(
    self: &mut Self,
    params: &mut BTreeMap<&str, Arc<str>>
  ) -> Result<()> {
    params.insert(
      "oauth_consumer_key",
      self.client_key.clone()
    );
    params.insert(
      "oauth_nonce",
      (
        (u64::MAX as f32 * self.rng.gen::<f32>()) as u64
      ).to_string().into()
    );
    params.insert(
      "oauth_timestamp",
      SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string()
        .into()
    );
    params.insert(
      "oauth_signature_method",
      "HMAC-SHA1".into()
    );
    params.insert(
      "oauth_version",
      "1.0".into()
    );

    Ok(())
  }

  pub(self) fn build_request(
    self: &Self,
    url: &str,
    params: Option<BTreeMap<&str, Arc<str>>>
  ) -> Result<Request<Body>> {
    let mut builder: request::Builder = Request::post(url)
      .header(
        header::USER_AGENT,
        "Rust@2021/hyper@0.14.26/hyper-rustls@0.24.0"
      )
      .header(
        header::ACCEPT_ENCODING,
        "gzip"
      )
      .header(
        header::ACCEPT,
        "*/*"
      )
      .header(
        header::CONNECTION,
        "keep-alive"
      )
      .header(
        header::CONTENT_TYPE,
        "application/json"
      )
      .header(
        header::CONTENT_LENGTH,
        0
      );

    if let Some(params) = params {
      builder = builder.header(
        header::AUTHORIZATION,
        format!(
          "OAuth {params}",
          params = params.into_iter()
            .map(
              |(k, v)| 
                format!(
                  r#"{key}="{value}""#,
                  key = urlencoding::encode(k),
                  value = urlencoding::encode(&v)
                ).into()
            )
            .collect::<Vec<Arc<str>>>()
            .join(", ")
        )
      );
    }

    let request: Request<Body> = builder.body(Body::empty())?;

    debug!("Updated headers: {:?}", request.headers());

    Ok(request)
  }

  async fn request(
    self: &mut Self,
    url: &str,
    mut params: BTreeMap<&str, Arc<str>>
  ) -> Result<Arc<str>> {
    self.apply_oauth_params(&mut params)?;

    self.apply_signture(
      url.split("?").next().ok_or(anyhow!("Split URL failed."))?,
      &mut params
    );

    let response: Response<Body> = self.http_client.request(
      self.build_request(url, Some(params))?
    ).await?;

    let content_length: usize = response.headers()
      .get(header::CONTENT_LENGTH)
      .ok_or(anyhow!("Get Content-Length failed."))?
      .to_str()?
      .parse()?;

    let mut body: String = String::with_capacity(content_length);

    debug!("Decoding response: {:?}", response.body());

    GzDecoder::new(
      &*body::to_bytes(response).await?
    ).read_to_string(&mut body)?;

    debug!("Response body: {:?}", body);

    Ok(body.into())
  }

  pub(self) fn decode_token(
    self: &Self,
    raw_token: &str
  ) -> Result<HashMap<Arc<str>, Arc<str>>> {
    debug!("Decoding token from response: {:?}", raw_token);

    let mut token: HashMap<Arc<str>, Arc<str>> = HashMap::new();
    for raw_token_pair in raw_token.split("&") {
      let (k, v): (&str, &str) = raw_token_pair
        .split("=")
        .collect_tuple()
        .ok_or(anyhow!("Decode token failed."))?;

      token.insert(k.into(), v.into());
    }

    debug!("Obtained token: {:?}", token);

    Ok(token)
  }

  async fn fetch_token(
    self: &mut Self,
    url: &str,
    params: BTreeMap<&str, Arc<str>>
  ) -> Result<HashMap<Arc<str>, Arc<str>>> {
    let raw_token: Arc<str> = self.request(url, params).await?;
    self.decode_token(&raw_token)
  }

  async fn get_access_token(
    self: &mut Self,
    verifier: &str,
    request_token: &str
  ) -> Result<(Option<Arc<str>> ,Option<Arc<str>>)> {
    let url: Arc<str> = format!(
      "https://api.twitter.com/oauth/access_token?oauth_verifier={oauth_verifier}&oauth_token={oauth_token}",
      oauth_verifier = verifier,
      oauth_token = request_token
    ).into();

    let token: HashMap<Arc<str>, Arc<str>> = self.fetch_token(
      &url, BTreeMap::new()
    ).await?;

    self.resource_owner_key = token.get("oauth_token").cloned();
    self.resource_owner_secret = token.get("oauth_token_secret").cloned();
    self.update_hash_key()?;

    Ok(
      (
        self.resource_owner_key.clone(),
        self.resource_owner_secret.clone()
      )
    )
  }
}
