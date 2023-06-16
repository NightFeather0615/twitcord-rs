use std::{collections::{HashMap, BTreeMap}, time::{SystemTime, UNIX_EPOCH}, io::Read};

use base64::{engine::general_purpose, Engine};
use flate2::read::GzDecoder;
use hmac::{Hmac, Mac};
use itertools::Itertools;
use log::debug;
use sha1::Sha1;
use hyper::{Client, Body, client::HttpConnector, Request, body, Response};
use hyper_rustls::HttpsConnector;
use rand::{thread_rng, rngs::ThreadRng, Rng};

type HmacSha1 = Hmac<Sha1>;

#[derive(Debug)]
pub struct TwitterClient {
  consumer_key: String,
  consumer_secret: String,
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
      consumer_key: consumer_key.clone(),
      consumer_secret: consumer_secret.clone(),
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

  pub async fn get_access_token(self: &mut Self, verifier: String) -> (String ,String) {
    let request: Request<Body> = Request::post(
      format!(
        "https://api.twitter.com/oauth/access_token?oauth_verifier={oauth_verifier}&oauth_token={oauth_token}",
        oauth_verifier = verifier,
        oauth_token = self.request_token.clone().expect("Get request token failed.")
      )
    )
      .header("User-Agent", "Rust@2021/hyper@0.14.26/hyper-rustls@0.24.0")
      .header("Accept-Encoding", "gzip")
      .header("Accept", "*/*")
      .header("Connection", "keep-alive")
      .header("Content-Length", 0)
      .body(Body::empty())
      .unwrap();

    let response: Response<Body> = self.oauth.http_client.request(request).await.unwrap();
    let mut raw_token: String = String::new();
    GzDecoder::new(&*body::to_bytes(response).await.unwrap()).read_to_string(&mut raw_token).unwrap();
    
    debug!("Decoding token from response: {:?}", raw_token);

    let mut token: HashMap<String, String> = HashMap::new();
    raw_token.split("&").for_each(|pair| {
      let (k, v) = pair.split("=").collect_tuple().unwrap();
      token.insert(k.to_string(), v.to_string());
    });

    self.access_token = token.get("oauth_token").cloned();
    self.access_token_secret = token.get("oauth_token_secret").cloned();

    self.oauth.update_hmac_sha1(self.access_token_secret.clone().expect("Get access token failed."));

    (self.access_token.clone().expect("Get access token failed."), self.access_token_secret.clone().expect("Get access token secret failed."))
  }

  pub async fn like(self: &mut Self, tweet_id: String) {
    let url: String = format!("https://api.twitter.com/1.1/favorites/create.json?id={}", tweet_id);

    let mut params: BTreeMap<&str, String> = BTreeMap::new();
    params.insert("oauth_token", self.access_token.clone().expect("Get access token failed."));

    self.oauth.apply_oauth_params(&mut params);

    debug!("Collected params: {:?}", params);
    
    params.insert("oauth_signature", self.oauth.generate_signture(&url, &params));

    let request: Request<Body> = Request::post(url)
      .header("User-Agent", "Rust@2021/hyper@0.14.26/hyper-rustls@0.24.0")
      .header("Accept-Encoding", "gzip")
      .header("Accept", "*/*")
      .header("Connection", "keep-alive")
      .header("Content-Type", "application/json")
      .header("Content-Length", 0)
      .header(
        "Authorization",
        format!(
          "OAuth {}",
          params.into_iter()
            .map(|(k, v)| format!(r#"{}="{}""#, urlencoding::encode(k), urlencoding::encode(&v)))
            .collect::<Vec<String>>()
            .join(", ")
        )
      )
      .body(Body::empty())
      .unwrap();

    debug!("Updated headers: {:?}", request.headers());
    
    let response: Response<Body> = self.oauth.http_client.request(request).await.unwrap();
    let mut raw_token: String = String::new();
    GzDecoder::new(&*body::to_bytes(response).await.unwrap()).read_to_string(&mut raw_token).unwrap();
    println!("{:?}", raw_token)
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
      hmac_sha1: HmacSha1::new_from_slice((client_secret.to_owned() + "&" + &resource_owner_secret.unwrap_or("".to_string())).as_bytes()).unwrap(),
      rng: thread_rng()
    }
  }

  fn update_hmac_sha1(self: &mut Self, resource_owner_secret: String) {
    self.hmac_sha1 = HmacSha1::new_from_slice((self.client_secret.to_owned() + "&" + &resource_owner_secret).as_bytes()).unwrap();
  }

  fn generate_signture(self: &mut Self, url: &str, params: &BTreeMap<&str, String>) -> String {
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

    signature
  }

  fn apply_oauth_params(self: &mut Self, params: &mut BTreeMap<&str, String>) {
    params.insert("oauth_consumer_key", self.client_key.clone());
    params.insert("oauth_nonce", ((u64::MAX as f32 * self.rng.gen::<f32>()) as u64).to_string());
    params.insert("oauth_timestamp", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string());
    params.insert("oauth_signature_method", "HMAC-SHA1".to_string());
    params.insert("oauth_version", "1.0".to_string());
  }

  async fn fetch_token(self: &mut Self, url: &str, mut params: BTreeMap<&str, String>) -> HashMap<String, String> {
    self.apply_oauth_params(&mut params);

    debug!("Collected params: {:?}", params);
    
    params.insert("oauth_signature", self.generate_signture(url, &params));

    let request: Request<Body> = Request::post(url)
      .header("User-Agent", "Rust@2021/hyper@0.14.26/hyper-rustls@0.24.0")
      .header("Accept-Encoding", "gzip")
      .header("Accept", "*/*")
      .header("Connection", "keep-alive")
      .header("Content-Length", 0)
      .header(
        "Authorization",
        format!(
          "OAuth {}",
          params.into_iter()
            .map(|(k, v)| format!(r#"{}="{}""#, urlencoding::encode(k), urlencoding::encode(&v)))
            .collect::<Vec<String>>()
            .join(", ")
        )
      )
      .body(Body::empty())
      .unwrap();

    debug!("Updated headers: {:?}", request.headers());

    let response: Response<Body> = self.http_client.request(request).await.unwrap();
    let mut raw_token: String = String::new();
    GzDecoder::new(&*body::to_bytes(response).await.unwrap()).read_to_string(&mut raw_token).unwrap();
    
    debug!("Decoding token from response: {:?}", raw_token);

    let mut token: HashMap<String, String> = HashMap::new();
    raw_token.split("&").for_each(|pair| {
      let (k, v) = pair.split("=").collect_tuple().unwrap();
      token.insert(k.to_string(), v.to_string());
    });

    debug!("Obtained token: {:?}", token);

    token
  }
}
