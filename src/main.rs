mod command;
mod core;


use std::{env, sync::Arc};

use dotenv::dotenv;
use rust_i18n::i18n;
use serenity::{
  async_trait,
  prelude::{
    EventHandler,
    Context,
    GatewayIntents
  },
  model::prelude::{
    interaction::Interaction,
    Ready,
    command::Command,
    Reaction,
    Message,
    ReactionType, Activity
  },
  Client,
  builder::{
    CreateApplicationCommands,
    CreateApplicationCommand
  },
  Error
};
use tracing::{Level, log::info};
use anyhow::{Result, anyhow};

use crate::core::{
  oauth::TwitterClient,
  utils::{get_tweet_id, match_locale}
};


i18n!(fallback = "en");


struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn reaction_add(
    &self,
    context: Context,
    reaction: Reaction
  ) {
    if let Ok(user) = reaction.user(&context.http).await {
      if user.bot {
        return;
      }

      let message: Result<Message, Error> = reaction.message(&context.http).await;
      
      if let Ok(message) = message {
        let tweet_id: Arc<str> = get_tweet_id(&message.content);
        if tweet_id.len() == message.content.len() {
          return;
        }
  
        let twitter_client: Result<TwitterClient> = TwitterClient::get_client(&context, user).await;
        
        if let Ok(mut twitter_client) = twitter_client {
          let result: Result<()> = match reaction.emoji.as_data().as_str() {
            "❤️" => twitter_client.like(&tweet_id).await,
            "🔁" => twitter_client.retweet(&tweet_id).await,
            "📡" => {
              let author_id: Result<Arc<str>> = twitter_client.get_author_id(&tweet_id).await;

              if let Ok(author_id) = author_id {
                twitter_client.follow(
                  &author_id
                ).await
              } else {
                Err(anyhow!("Fetch tweet author ID failed."))
              }
            },
            _ => Err(anyhow!("Unknown action."))
          };

          
          if let Err(why) = result {
            info!("Reaction add error: {}", why);
          }
        }
      }
    }
  }

  async fn reaction_remove(
    &self,
    context: Context,
    reaction: Reaction
  ) {
    if let Ok(user) = reaction.user(&context.http).await {
      if user.bot {
        return;
      }

      let message: Result<Message, Error> = reaction.message(&context.http).await;
      
      if let Ok(message) = message {
        let tweet_id: Arc<str> = get_tweet_id(&message.content);
        if tweet_id.len() == message.content.len() {
          return;
        }
  
        let twitter_client: Result<TwitterClient> = TwitterClient::get_client(
          &context,
          user
        ).await;
        
        if let Ok(mut twitter_client) = twitter_client {
          let result: Result<()> = match reaction.emoji.as_data().as_str() {
            "❤️" => twitter_client.unlike(&tweet_id).await,
            "🔁" => twitter_client.unretweet(&tweet_id).await,
            "📡" => {
              let author_id: Result<Arc<str>> = twitter_client.get_author_id(&tweet_id).await;

              if let Ok(author_id) = author_id {
                twitter_client.unfollow(
                  &author_id
                ).await
              } else {
                Err(anyhow!("Fetch tweet author ID failed."))
              }
            },
            _ => Err(anyhow!("Unknown action."))
          };

          
          if let Err(why) = result {
            info!("Reaction remove error: {}", why);
          }
        }
      }
    }
  }

  async fn message(
    &self,
    context: Context,
    message: Message
  ) {
    let tweet_id: Arc<str> = get_tweet_id(&message.content);
    if tweet_id.len() == message.content.len() {
      return;
    }

    for emoji in ["❤️", "🔁", "📡"] {
      if let Err(why) = message.react(&context.http, ReactionType::Unicode(emoji.to_string())).await {
        info!("Reaction add error: {}", why);
        break;
      };
    }
  }

  async fn interaction_create(
    &self,
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

  async fn ready(&self, context: Context, ready: Ready) {
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
    .pretty()
    .init();
  
  dotenv().ok();

  let token: String = env::var("DISCORD_BOT_TOKEN")
    .expect("Expected a token in the environment");

  let intents: GatewayIntents = GatewayIntents::DIRECT_MESSAGES
    | GatewayIntents::GUILD_MEMBERS
    | GatewayIntents::GUILD_MESSAGE_REACTIONS
    | GatewayIntents::GUILD_MESSAGES
    | GatewayIntents::MESSAGE_CONTENT;

  let mut client: Client = Client::builder(token, intents)
    .event_handler(Handler)
    .await
    .expect("Error creating client");

  if let Err(why) = client.start().await {
    info!("Client error: {:?}", why);
  }
}
