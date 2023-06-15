use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}};

use base64::{engine::general_purpose, Engine};
use crypto::{hmac::Hmac, sha1::Sha1, mac::Mac};
use hyper::{Client, Body, client::HttpConnector};
use hyper_rustls::HttpsConnector;
use rand::{thread_rng, rngs::ThreadRng, Rng};


pub struct OAuthHandler<'a> {
  consumer_key: &'a str,
  consumer_secret: &'a str,
  request_token: HashMap<String, String>,
  oauth: OAuthSession<'a>
}

impl<'a> OAuthHandler<'a> {
  pub fn new(consumer_key: &'a str, consumer_secret: &'a str) -> OAuthHandler<'a> {
    OAuthHandler {
      consumer_key: consumer_key.clone(),
      consumer_secret: consumer_secret.clone(),
      request_token: HashMap::new(),
      oauth: OAuthSession::new(consumer_key, consumer_secret)
    }
  }

  pub fn get_authorization_url(self: &mut Self) -> String {
    self.oauth.fetch_request_token();
    "https://api.twitter.com/oauth/authorize".to_string()
  }

  pub fn get_access_token(self: &Self, verifier: String) {
    let url = "https://api.twitter.com/oauth/access_token".to_string();

  }
}

struct OAuthSession<'a> {
  client_key: &'a str,
  client_secret: &'a str,
  http_client: Client<HttpsConnector<HttpConnector>, Body>,
  hmac_sha1: Hmac<Sha1>,
  rng: ThreadRng
}

impl<'a> OAuthSession<'a> {
  pub fn new(client_key: &'a str, client_secret: &'a str) -> OAuthSession<'a> {
    let https: HttpsConnector<HttpConnector> = hyper_rustls::HttpsConnectorBuilder::new()
      .with_native_roots()
      .https_only()
      .enable_http1()
      .build();

    OAuthSession {
      client_key,
      client_secret: client_secret.clone(),
      http_client: Client::builder().build(https),
      hmac_sha1: Hmac::new(Sha1::new(), (client_secret.to_owned() + "&").as_bytes()),
      rng: thread_rng()
    }
  }

  fn fetch_request_token(self: &mut Self) {
    let url: &str = "https://api.twitter.com/oauth/request_token";

    let timestamp: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let params: String = format!(
      "oauth_consumer_key={oauth_consumer_key}&oauth_nonce={oauth_nonce}&oauth_signature_method={oauth_signature_method}&oauth_timestamp={oauth_timestamp}&oauth_version={oauth_version}",
      oauth_consumer_key = self.client_key,
      oauth_nonce = ((u16::MAX as f32 * self.rng.gen::<f32>() * timestamp as f32) as u64).to_string(),
      oauth_signature_method = "HMAC-SHA1",
      oauth_timestamp = timestamp.to_string(),
      oauth_version = "1.0"
    );

    let signature_base: String = format!(
      "{method}&{url}&{params}",
      method = urlencoding::encode("POST"),
      url = urlencoding::encode(&url),
      params = urlencoding::encode(&params)
    );

    self.hmac_sha1.reset();
    self.hmac_sha1.input(signature_base.as_bytes());

    println!("{:?}", general_purpose::STANDARD.encode(self.hmac_sha1.result().code()));
  }
}
