mod oauth;

use oauth::TwitterClient;
use dotenv::dotenv;
use std::{env, io};

#[tokio::main]
async fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).try_init().unwrap();
    dotenv().ok();

    let mut auth: TwitterClient = TwitterClient::new(
        env::var("TWITTER_CONSUMER_KEY").unwrap().into(),
        env::var("TWITTER_CONSUMER_SECRET").unwrap().into(),
        None,
        None
    );
    println!("{:?}", auth.get_authorization_url().await);

    let mut input_text: String = String::new();
    io::stdin()
        .read_line(&mut input_text)
        .expect("failed to read from stdin");

    auth.get_access_token(input_text.trim()).await;

    auth.retweet("1669464988161908738").await;

    let author_id: String = auth.get_author_id("1669464988161908738").await;
    auth.follow(&author_id).await;
}