use std::sync::{OnceLock, Arc};

use regex::Regex;
use rust_i18n::t;
use serenity::{
  model::prelude::{
    PrivateChannel, interaction::{
      application_command::ApplicationCommandInteraction,
      InteractionResponseType
    }
  },
  prelude::Context,
  builder::{
    GetMessages,
    EditMessage,
    CreateInteractionResponse,
    CreateInteractionResponseData
  },
  Error,
};
use anyhow::Result;


static TWITTER_POST_ID_REGEX: OnceLock<Regex> = OnceLock::new();

pub static TWITTER_CONSUMER_KEY: OnceLock<Arc<str>> = OnceLock::new();
pub static TWITTER_CONSUMER_SECRET: OnceLock<Arc<str>> = OnceLock::new();

pub static EMBED_INFO_COLOR: u32 = 0x3983f2;
pub static EMBED_ERROR_COLOR: u32 = 0xeca42c;


pub fn get_tweet_id(url: &str) -> Arc<str> {
  TWITTER_POST_ID_REGEX.get_or_init(
    || Regex::new(
      r#"(?:https|http)://(www\.)?(?:twitter|fxtwitter|vxtwitter)\.com/[A-Za-z0-9_][^ =&/:]{1,15}/status/(?P<tweet_id>[0-9].*)"#
    ).expect("Regex init failed.")
  )
  .replace(url, "$tweet_id")
  .into()
}

pub fn match_locale(discord_locale: &str) -> String {
  match discord_locale {
    "zh-TW" => "zh-TW".to_string(),
    "zh-CN" => "zh-CN".to_string(),
    _ => "en".to_string()
  }
}

pub async fn check_dm(
  context: &Context,
  interaction: &ApplicationCommandInteraction
) -> Result<bool> {
  let is_dm: bool = context.http.get_channel(
    *interaction.channel_id.as_u64()
  ).await?.private().is_some();
  
  if is_dm {
    interaction.defer(&context.http).await?;
  } else {
    interaction.create_interaction_response(
      &context.http,
      |response: &mut CreateInteractionResponse<'_>| {
        response
          .kind(
            InteractionResponseType::ChannelMessageWithSource
          )
          .interaction_response_data(
            |message: &mut CreateInteractionResponseData<'_>| {
              message
                .ephemeral(true)
                .content(
                  t!(
                    "command.check-dm",
                    locale = &interaction.locale
                  )
                )
            }
          )
      }
    ).await?;
  }

  Ok(is_dm)
}

pub async fn clean_up_dm(
  context: &Context,
  dm_channel: &PrivateChannel
) -> Result<(), Error> {
  for message in dm_channel.pins(&context.http).await? {
    message.unpin(&context.http).await?
  }

  for message in dm_channel.messages(
    &context.http,
    |retriever: &mut GetMessages| retriever
  ).await?.iter_mut() {
    if message.author.bot && message.content.contains("Twitter User Access Token") {
      message.edit(
        &context.http,
        |message: &mut EditMessage<'_>| {
          message
            .content(
              "[Disconnected] Twitter User Access Token\n`[Access Token cancelled]`\n`[Access Token Secret cancelled]`"
            )
        }
      ).await?;
    }
  }

  Ok(())
}
