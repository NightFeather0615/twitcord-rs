mod command;
mod core;


use std::{env, sync::Arc, time::Duration};

use dotenv::dotenv;
use rust_i18n::i18n;
use serenity::{
  async_trait,
  prelude::{
    EventHandler,
    Context,
    GatewayIntents
  },
  model::{
    prelude::{
      interaction::Interaction,
      Ready,
      command::Command,
      Reaction,
      Message,
      ReactionType,
      Activity
    },
    user::User
  },
  Client,
  builder::{
    CreateApplicationCommands,
    CreateApplicationCommand
  }
};
use tokio::{task, time::sleep};
use tracing::{Level, log::info};
use anyhow::{Result, anyhow};

use crate::core::{
  oauth::TwitterClient,
  utils::{get_tweet_id, match_locale},
  cache::{AccessTokenCache, MAX_AGE}
};


i18n!(fallback = "en");


struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn reaction_add(
    self: &Self,
    context: Context,
    reaction: Reaction
  ) {
    let user: User = match reaction.user(&context.http).await {
      Ok(user) => user,
      Err(why) => { info!("{:?}", why); return; }
    };

    if user.bot {
      return;
    }

    let message: Message = match reaction.message(&context.http).await {
      Ok(message) => message,
      Err(why) => { info!("{:?}", why); return; }
    };
    
    let tweet_id: Arc<str> = get_tweet_id(&message.content);

    if tweet_id.len() == message.content.len() {
      return;
    }

    let mut twitter_client: TwitterClient = match TwitterClient::get_client(&context, user).await {
      Ok(twitter_client) => twitter_client,
      Err(why) => { info!("{:?}", why); return; }
    };
    
    let result: Result<()> = match reaction.emoji.as_data().as_str() {
      "❤️" => twitter_client.like(&tweet_id).await,
      "🔁" => twitter_client.retweet(&tweet_id).await,
      "📡" => match twitter_client.get_author_id(&tweet_id).await {
        Ok(author_id) => twitter_client.follow(&author_id).await,
        Err(why) => { info!("{:?}", why); return; }
      },
      _ => Err(anyhow!("Unknown action."))
    };

    if let Err(why) = result {
      info!("Event reaction_remove error: {}", why);
    }
  }

  async fn reaction_remove(
    self: &Self,
    context: Context,
    reaction: Reaction
  ) {
    let user: User = match reaction.user(&context.http).await {
      Ok(user) => user,
      Err(why) => { info!("{:?}", why); return; }
    };

    if user.bot {
      return;
    }

    let message: Message = match reaction.message(&context.http).await {
      Ok(message) => message,
      Err(why) => { info!("{:?}", why); return; }
    };
    
    let tweet_id: Arc<str> = get_tweet_id(&message.content);

    if tweet_id.len() == message.content.len() {
      return;
    }

    let mut twitter_client: TwitterClient = match TwitterClient::get_client(&context, user).await {
      Ok(twitter_client) => twitter_client,
      Err(why) => { info!("{:?}", why); return; }
    };
    
    let result: Result<()> = match reaction.emoji.as_data().as_str() {
      "❤️" => twitter_client.unlike(&tweet_id).await,
      "🔁" => twitter_client.unretweet(&tweet_id).await,
      "📡" => match twitter_client.get_author_id(&tweet_id).await {
        Ok(author_id) => twitter_client.unfollow(&author_id).await,
        Err(why) => { info!("{:?}", why); return; }
      },
      _ => Err(anyhow!("Unknown action."))
    };

    if let Err(why) = result {
      info!("Event reaction_remove error: {}", why);
    }
  }

  async fn message(
    self: &Self,
    context: Context,
    message: Message
  ) {
    let tweet_id: Arc<str> = get_tweet_id(&message.content);

    if tweet_id.len() == message.content.len() {
      return;
    }

    for emoji in ["❤️", "🔁", "📡"] {
      match message.react(
        &context.http,
        ReactionType::Unicode(emoji.to_string())
      ).await {
        Ok(_) => (),
        Err(why) => { info!("{:?}", why); return; }
      }
    }
  }

  async fn interaction_create(
    self: &Self,
    context: Context,
    interaction: Interaction
  ) {
    if let Interaction::ApplicationCommand(
      mut command
    ) = interaction {
      info!("Received command interaction: {:#?}", command);

      command.locale = match_locale(&command.locale);

      let result: Result<()> = match command.data.name.as_str() {
        "connect" => command::connect::execute(&context, &command).await,
        "disconnect" => command::disconnect::execute(&context, &command).await,
        "support" => command::support::execute(&context, &command).await,
        "invite" => command::invite::execute(&context, &command).await,
        _ => Err(anyhow!("Interaction not found."))
      };

      if let Err(why) = result {
        info!("Command interaction error: {}", why);
      }
    }
  }

  async fn ready(self: &Self, context: Context, ready: Ready) {
    info!("Logged in as {}#{}", ready.user.name, ready.user.discriminator);

    let commands: Vec<Command> = Command::set_global_application_commands(
      &context.http,
      |commands: &mut CreateApplicationCommands| {
        commands
          .create_application_command(
            |command: &mut CreateApplicationCommand| {
              command::connect::register(command)
            }
          )
          .create_application_command(
            |command: &mut CreateApplicationCommand| {
              command::disconnect::register(command)
            }
          )
          .create_application_command(
            |command: &mut CreateApplicationCommand| {
              command::support::register(command)
            }
          )
          .create_application_command(
            |command: &mut CreateApplicationCommand| {
              command::invite::register(command)
            }
          )
      }
    ).await.expect("Register commands failed.");

    for command in commands {
      info!("Registered command {}", command.name);
    }

    context.online().await;
    context.set_activity(
      Activity::watching("🕊️ | /connect")
    ).await;
  }
}


#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(Level::INFO)
    .init();
  
  dotenv().ok();

  let token: String = env::var("DISCORD_BOT_TOKEN")
    .expect("Expected a token in the environment");

  let intents: GatewayIntents = GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::GUILD_MEMBERS
    | GatewayIntents::GUILD_MESSAGE_REACTIONS
    | GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT;

  AccessTokenCache::init();

  task::spawn(
    async {
      loop {
        sleep(Duration::from_secs(MAX_AGE / 2)).await;
        info!("Clean up access token cache.");
        AccessTokenCache::init().clean_up().await;
      }
    }
  );

  let mut client: Client = Client::builder(token, intents)
    .event_handler(Handler)
    .await
    .expect("Error creating client");

  if let Err(why) = client.start().await {
    info!("Client error: {:?}", why);
  }
}
