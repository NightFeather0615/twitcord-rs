use std::collections::HashMap;

use hyper::{Client, Body, client::HttpConnector};
use hyper_rustls::HttpsConnector;

pub struct OAuthHandler {
  consumer_key: String,
  consumer_secret: String,
  request_token: HashMap<String, String>,
  oauth: OAuthSession
}

impl OAuthHandler {
  pub fn new(consumer_key: String, consumer_secret: String) -> OAuthHandler {
    OAuthHandler {
      consumer_key: consumer_key.clone(),
      consumer_secret: consumer_secret.clone(),
      request_token: HashMap::new(),
      oauth: OAuthSession::new(consumer_key, consumer_secret)
    }
  }

  pub fn get_authorization_url(self: &Self) -> String {
    "https://api.twitter.com/oauth/authorize".to_string()
  }

  pub fn get_access_token(self: &Self, verifier: String) {
    let url = "https://api.twitter.com/oauth/access_token".to_string();

  }
}

struct OAuthSession {
  client_key: String,
  client_secret: String,
  http_client: Client<HttpsConnector<HttpConnector>, Body>
}

impl OAuthSession {
  pub fn new(client_key: String, client_secret: String) -> OAuthSession {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
      .with_native_roots()
      .https_only()
      .enable_http1()
      .build();

    OAuthSession {
      client_key,
      client_secret,
      http_client: Client::builder().build(https)
    }
  }

  fn fetch_request_token(self: &Self) {
    let url = "https://api.twitter.com/oauth/request_token".to_string();
  }
}
