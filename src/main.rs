mod command;
mod core;


use std::{env, time::Duration};

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
    }
  },
  Client,
  builder::{
    CreateApplicationCommands,
    CreateApplicationCommand
  }
};
use tokio::{task, time::sleep};
use tracing::{Level, log::{info, error}};
use anyhow::{Result, anyhow};

use crate::core::{
  utils::{get_first_tweet_id, match_locale, process_reaction},
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
    let (mut twitter_client, tweet_id) = match process_reaction(&context, &reaction).await {
      Some((twitter_client, tweet_id)) => (twitter_client, tweet_id),
      None => return
    };
    
    let result: Result<()> = match reaction.emoji.as_data().as_str() {
      "❤️" => twitter_client.like(&tweet_id).await,
      "🔁" => twitter_client.retweet(&tweet_id).await,
      "📡" => match twitter_client.get_author_id(&tweet_id).await {
        Ok(author_id) => twitter_client.follow(&author_id).await,
        Err(why) => { error!("{:?}", why); return; }
      },
      _ => return
    };

    info!(
      "Invoke action `{action}` | Tweet: {tweet_id} | User: {user_id} | Guild: {guild_id}",
      action = reaction.emoji.as_data().as_str(),
      tweet_id = tweet_id,
      user_id = match reaction.user_id {
        Some(user_id) => *user_id.as_u64(),
        None => 0
      },
      guild_id = match reaction.guild_id {
        Some(guild_id) => *guild_id.as_u64(),
        None => 0
      },
    );

    if let Err(why) = result {
      error!("ReactionAdd error: {}", why);
    }
  }

  async fn reaction_remove(
    self: &Self,
    context: Context,
    reaction: Reaction
  ) {
    let (mut twitter_client, tweet_id) = match process_reaction(&context, &reaction).await {
      Some((twitter_client, tweet_id)) => (twitter_client, tweet_id),
      None => return
    };
    
    let result: Result<()> = match reaction.emoji.as_data().as_str() {
      "❤️" => twitter_client.unlike(&tweet_id).await,
      "🔁" => twitter_client.unretweet(&tweet_id).await,
      "📡" => match twitter_client.get_author_id(&tweet_id).await {
        Ok(author_id) => twitter_client.unfollow(&author_id).await,
        Err(why) => { error!("{:?}", why); return; }
      },
      _ => return
    };

    info!(
      "Revoke action `{action}` | Tweet: {tweet_id} | User: {user_id} | Guild: {guild_id}",
      action = reaction.emoji.as_data().as_str(),
      tweet_id = tweet_id,
      user_id = match reaction.user_id {
        Some(user_id) => *user_id.as_u64(),
        None => 0
      },
      guild_id = match reaction.guild_id {
        Some(guild_id) => *guild_id.as_u64(),
        None => 0
      },
    );

    if let Err(why) = result {
      error!("ReactionRemove error: {}", why);
    }
  }

  async fn message(
    self: &Self,
    context: Context,
    message: Message
  ) {
    match message.channel(&context.http).await {
      Ok(channel) => {
        if channel.private().is_some() {
          return;
        }
      },
      Err(why) => { error!("Fetch channel error: {:?}", why); return; }
    }

    match get_first_tweet_id(&message.content) {
      Some(tweet_id) => {
        for emoji in ["❤️", "🔁", "📡"] {
          match message.react(
            &context.http,
            ReactionType::Unicode(emoji.to_string())
          ).await {
            Ok(_) => (),
            Err(why) => { error!("Apply reaction error: {:?}", why); return; }
          }
        }
        
        info!(
          "Applied reaction | Tweet: {tweet_id} | User: {user_id} | Guild: {guild_id}",
          tweet_id = tweet_id,
          user_id = message.author.id.as_u64(),
          guild_id = match message.guild_id {
            Some(guild_id) => *guild_id.as_u64(),
            None => 0
          },
        )
      },
      None => ()
    }
  }

  async fn interaction_create(
    self: &Self,
    context: Context,
    interaction: Interaction
  ) {
    if let Interaction::ApplicationCommand(mut interaction) = interaction {
      info!(
        "Received ApplicationCommand `{name}` | User: {user_id} | Guild: {guild_id}",
        name = interaction.data.name,
        guild_id = match interaction.guild_id {
          Some(guild_id) => *guild_id.as_u64(),
          None => 0
        },
        user_id = interaction.user.id.as_u64()
      );

      interaction.locale = match_locale(&interaction.locale);

      let result: Result<()> = match interaction.data.name.as_str() {
        "connect" => command::connect::execute(&context, &interaction).await,
        "disconnect" => command::disconnect::execute(&context, &interaction).await,
        "support" => command::support::execute(&context, &interaction).await,
        "invite" => command::invite::execute(&context, &interaction).await,
        _ => Err(anyhow!("Interaction not found."))
      };

      if let Err(why) = result {
        error!("ApplicationCommand error: {:?}", why);
      }
    }
  }

  async fn ready(self: &Self, context: Context, ready: Ready) {
    info!("Logged in as `{}#{}`", ready.user.name, ready.user.discriminator);

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
      info!("Registered command `{}`", command.name);
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

  task::spawn(
    async {
      loop {
        sleep(Duration::from_secs(MAX_AGE / 2)).await;
        AccessTokenCache::get().clean_up().await;
      }
    }
  );

  let mut client: Client = Client::builder(token, intents)
    .event_handler(Handler)
    .await
    .expect("Error creating client");

  if let Err(why) = client.start().await {
    error!("Client error: {:?}", why);
  }
}
