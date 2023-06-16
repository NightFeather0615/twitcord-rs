use std::{collections::{HashMap, BTreeMap}, time::{SystemTime, UNIX_EPOCH}, io::Read, sync::OnceLock};

use base64::{engine::general_purpose, Engine};
use flate2::read::GzDecoder;
use hmac::{Hmac, Mac};
use itertools::Itertools;
use log::debug;
use regex::Regex;
use sha1::Sha1;
use hyper::{Client, Body, client::HttpConnector, Request, body, Response, http::request};
use hyper_rustls::HttpsConnector;
use rand::{thread_rng, rngs::ThreadRng, Rng};

type HmacSha1 = Hmac<Sha1>;

static LOOKUP_USER_ID_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Debug)]
pub struct TwitterClient {
  request_token: Option<String>,
  request_token_secret: Option<String>,
  access_token: Option<String>,
  access_token_secret: Option<String>,
  oauth: OAuthSession
}

impl TwitterClient {
  pub fn new(
    consumer_key: String,
    consumer_secret: String,
    access_token: Option<String>,
    access_token_secret: Option<String>
  ) -> TwitterClient {
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
      )
    }
  }

  pub async fn get_authorization_url(self: &mut Self) -> String {
    let token: HashMap<String, String> = self.oauth.fetch_token(
      "https://api.twitter.com/oauth/request_token",
      BTreeMap::new()
    ).await;

    self.request_token = token.get("oauth_token").cloned();
    self.request_token_secret = token.get("oauth_token_secret").cloned();

    format!(
      "https://api.twitter.com/oauth/authorize?oauth_token={oauth_token}",
      oauth_token = self.request_token.clone().expect("Get request token failed.")
    )
  }

  pub async fn get_access_token(self: &mut Self, verifier: &str) -> (&str, &str) {
    (self.access_token, self.access_token_secret) = self.oauth.get_access_token(
      verifier,
      self.request_token.as_ref().expect("Get request token failed.")
    ).await;
    (
      self.access_token.as_ref().expect("Get access token failed."),
      self.access_token_secret.as_ref().expect("Get access token secret failed.")
    )
  }

  pub async fn like(self: &mut Self, tweet_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/favorites/create.json?id={}", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    params.insert(
      "id",
      tweet_id.to_string()
    );
    
    self.oauth.request(&url, params).await;
  }

  pub async fn unlike(self: &mut Self, tweet_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/favorites/destroy.json?id={}", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    params.insert(
      "id",
      tweet_id.to_string()
    );
    
    self.oauth.request(&url, params).await;
  }

  pub async fn retweet(self: &mut Self, tweet_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/statuses/retweet/{}.json", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );

    self.oauth.request(&url, params).await;
  }

  pub async fn unretweet(self: &mut Self, tweet_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/statuses/unretweet/{}.json", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    
    self.oauth.request(&url, params).await;
  }

  pub async fn get_author_id(self: &mut Self, tweet_id: &str) -> String {
    let url: String = format!("https://api.twitter.com/1.1/statuses/lookup.json?id={}&trim_user=true", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    params.insert(
      "id",
      tweet_id.to_string()
    );
    params.insert(
      "trim_user",
      "true".to_string()
    );

    LOOKUP_USER_ID_REGEX
      .get_or_init(|| Regex::new(r#".*"user":\{"id":(?P<id>[0-9]{19}),"id_str":"[0-9]{19}"\}.*"#)
      .expect("Regex init failed."))
      .replace(&self.oauth.request(&url, params).await, "$id")
      .to_string()
  }

  pub async fn follow(self: &mut Self, user_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/friendships/create.json?user_id={}", user_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    params.insert(
      "user_id",
      user_id.to_string()
    );
    
    self.oauth.request(&url, params).await;
  }

  pub async fn unfollow(self: &mut Self, user_id: &str) {
    let url: String = format!("https://api.twitter.com/1.1/friendships/destroy.json?user_id={}", user_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert(
      "oauth_token",
      self.access_token.clone().expect("Get access token failed.")
    );
    params.insert(
      "user_id",
      user_id.to_string()
    );
    
    self.oauth.request(&url, params).await;
  }
}

#[derive(Debug)]
struct OAuthSession {
  client_key: String,
  client_secret: String,
  resource_owner_key: Option<String>,
  resource_owner_secret: Option<String>,
  http_client: Client<HttpsConnector<HttpConnector>, Body>,
  hmac_sha1: HmacSha1,
  rng: ThreadRng
}

impl OAuthSession {
  fn new(
    client_key: String,
    client_secret: String,
    resource_owner_key: Option<String>,
    resource_owner_secret: Option<String>
  ) -> OAuthSession {
    let https_connector: HttpsConnector<HttpConnector> = hyper_rustls::HttpsConnectorBuilder::new()
      .with_native_roots()
      .https_only()
      .enable_http1()
      .build();

    OAuthSession {
      client_key,
      client_secret: client_secret.clone(),
      resource_owner_key,
      resource_owner_secret: resource_owner_secret.clone(),
      http_client: Client::builder().build(https_connector),
      hmac_sha1: HmacSha1::new_from_slice(
        format!(
          "{}&{}",
          urlencoding::encode(&client_secret),
          urlencoding::encode(&resource_owner_secret.unwrap_or("".to_string()))
        ).as_bytes()
      ).unwrap(),
      rng: thread_rng()
    }
  }

  pub(self) fn update_hash_key(self: &mut Self) {
    self.hmac_sha1 = HmacSha1::new_from_slice(
      format!(
        "{}&{}",
        urlencoding::encode(&self.client_secret),
        urlencoding::encode(&self.resource_owner_secret.clone().unwrap_or("".to_string()))
      ).as_bytes()
    ).unwrap();
  }

  pub(self) fn apply_signture(self: &mut Self, url: &str, params: &mut BTreeMap<&str, String>) {
    debug!("Collected params: {:?}", params);

    let normalized_params: String = params.iter()
      .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
      .collect::<Vec<String>>()
      .join("&");

    debug!("Normalized params: {:?}", normalized_params);

    let signature_base: String = format!(
      "{method}&{url}&{params}",
      method = urlencoding::encode("POST"),
      url = urlencoding::encode(url),
      params = urlencoding::encode(&normalized_params)
    );

    debug!("Signature base string: {:?}", signature_base);

    self.hmac_sha1.update(signature_base.as_bytes());
    let signature: String = general_purpose::STANDARD.encode(self.hmac_sha1.finalize_reset().into_bytes());

    debug!("Signature: {:?}", signature);

    params.insert(
      "oauth_signature",
      signature
    );
  }

  pub(self) fn apply_oauth_params(self: &mut Self, params: &mut BTreeMap<&str, String>) {
    params.insert(
      "oauth_consumer_key",
      self.client_key.to_string()
    );
    params.insert(
      "oauth_nonce",
      ((u64::MAX as f32 * self.rng.gen::<f32>()) as u64).to_string()
    );
    params.insert(
      "oauth_timestamp",
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string()
    );
    params.insert(
      "oauth_signature_method",
      "HMAC-SHA1".to_string()
    );
    params.insert(
      "oauth_version",
      "1.0".to_string()
    );
  }

  pub(self) fn build_request(self: &Self, url: &str, params: Option<BTreeMap<&str, String>>) -> Request<Body> {
    let mut builder: request::Builder = Request::post(url)
      .header("User-Agent", "Rust@2021/hyper@0.14.26/hyper-rustls@0.24.0")
      .header("Accept-Encoding", "gzip")
      .header("Accept", "*/*")
      .header("Connection", "keep-alive")
      .header("Content-Type", "application/json")
      .header("Content-Length", 0);

    if let Some(params) = params {
      builder = builder.header(
        "Authorization",
        format!(
          "OAuth {}",
          params.into_iter()
            .map(|(k, v)| format!(r#"{}="{}""#, urlencoding::encode(k), urlencoding::encode(&v)))
            .collect::<Vec<String>>()
            .join(", ")
        )
      );
    }

    let request: Request<Body> = builder.body(Body::empty()).unwrap();

    debug!("Updated headers: {:?}", request.headers());

    request
  }

  async fn request(self: &mut Self, url: &str, mut params: BTreeMap<&str, String>) -> String {
    self.apply_oauth_params(&mut params);

    self.apply_signture(url.split("?").next().unwrap(), &mut params);

    let response: Response<Body> = self.http_client.request(
      self.build_request(url, Some(params))
    ).await.unwrap();

    let mut body: String = String::new();

    GzDecoder::new(&*body::to_bytes(response).await.unwrap()).read_to_string(&mut body).unwrap();

    body
  }

  pub(self) fn decode_token(self: &Self, raw_token: &str) -> HashMap<String, String> {
    debug!("Decoding token from response: {:?}", raw_token);

    let mut token: HashMap<String, String> = HashMap::new();
    raw_token.split("&").for_each(|pair: &str| {
      let (k, v) = pair.split("=").collect_tuple().unwrap();
      token.insert(k.to_string(), v.to_string());
    });

    debug!("Obtained token: {:?}", token);

    token
  }

  async fn fetch_token(self: &mut Self, url: &str, params: BTreeMap<&str, String>) -> HashMap<String, String> {
    let raw_token: String = self.request(url, params).await;
    self.decode_token(&raw_token)
  }

  async fn get_access_token(self: &mut Self, verifier: &str, request_token: &str) -> (Option<String> ,Option<String>) {
    let url: String = format!(
      "https://api.twitter.com/oauth/access_token?oauth_verifier={oauth_verifier}&oauth_token={oauth_token}",
      oauth_verifier = verifier,
      oauth_token = request_token
    );

    let token: HashMap<String, String> = self.fetch_token(&url, BTreeMap::new()).await;

    self.resource_owner_key = token.get("oauth_token").cloned();
    self.resource_owner_secret = token.get("oauth_token_secret").cloned();
    self.update_hash_key();

    (self.resource_owner_key.clone(), self.resource_owner_secret.clone())
  }
}
