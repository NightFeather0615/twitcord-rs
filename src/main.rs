mod oauth;
mod utils;
mod commands;


use std::env;

use dotenv::dotenv;
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
    command::Command},
  Client,
  builder::{
    CreateApplicationCommands,
    CreateApplicationCommand
  }
};
use tracing::{Level, log::info};
use anyhow::{Result, anyhow};


struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn interaction_create(
    &self, context: Context,
    interaction: Interaction
  ) {
    if let Interaction::ApplicationCommand(
      command
    ) = interaction {
      println!("Received command interaction: {:#?}", command);

      let result: Result<()> = match command.data.name.as_str() {
        "connect" => commands::connect::execute(&context, &command).await,
        _ => Err(anyhow!("Interaction not found."))
      };

      if let Err(why) = result {
        println!("Command interaction error: {}", why);
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
              commands::connect::register(command)
            }
          )
      }
    ).await.expect("Register commands failed.");

    for command in commands {
      info!("Registered command {}", command.name);
    }
  }
}


#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(Level::DEBUG)
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
