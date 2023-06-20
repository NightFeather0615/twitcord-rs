use std::sync::{OnceLock, Arc};

use regex::Regex;
use rust_i18n::t;
use serenity::{
  model::{
    prelude::{
      PrivateChannel, interaction::{
        application_command::ApplicationCommandInteraction,
        InteractionResponseType
      },
      Message,
      Reaction
    },
    user::User
  },
  prelude::Context,
  builder::{
    GetMessages,
    EditMessage,
    CreateInteractionResponse,
    CreateInteractionResponseData,
    CreateEmbed
  },
  Error, utils::Color,
};
use anyhow::Result;
use tracing::log::error;

use super::oauth::TwitterClient;


static TWITTER_POST_ID_REGEX: OnceLock<Regex> = OnceLock::new();

pub static EMBED_INFO_COLOR: u32 = 0x3983f2;
pub static EMBED_ERROR_COLOR: u32 = 0xeca42c;


pub async fn process_reaction(
  context: &Context,
  reaction: &Reaction
) -> Option<(TwitterClient, Arc<str>)> {
  let user: User = match reaction.user(&context.http).await {
    Ok(user) => user,
    Err(why) => { error!("{:?}", why); return None; }
  };

  if user.bot {
    return None;
  }

  let message: Message = match reaction.message(&context.http).await {
    Ok(message) => message,
    Err(why) => { error!("{:?}", why); return None; }
  };
  
  let tweet_id: Arc<str> = match get_first_tweet_id(&message.content) {
    Some(tweet_id) => tweet_id,
    None => return None
  };

  let twitter_client: TwitterClient = match TwitterClient::get_client(&context, user).await {
    Ok(twitter_client) => twitter_client,
    Err(why) => { error!("{:?}", why); return None; }
  };

  Some((twitter_client, tweet_id))
}

pub fn get_first_tweet_id(message: &str) -> Option<Arc<str>> {
  let tweet_id: Arc<str> = TWITTER_POST_ID_REGEX.get_or_init(
    || {
      Regex::new(
        r#".*(?:https|http)://(?:www\.)?(?:twitter|fxtwitter|vxtwitter)\.com/[A-Za-z0-9_]{1,15}/status/(?P<tweet_id>[0-9]*).*"#
      ).expect("Regex init failed.")
    }
  )
    .replace(message, "$tweet_id")
    .into();

  if tweet_id.len() == message.len() {
    return None;
  }

  Some(tweet_id)
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
                .embed(
                  |embed: &mut CreateEmbed| {
                    embed
                      .color(Color::new(EMBED_INFO_COLOR))
                      .title(
                        t!(
                          "command.check-dm-embed.title",
                          locale = &interaction.locale
                        )
                      )
                      .description(
                        t!(
                          "command.check-dm-embed.description",
                          locale = &interaction.locale
                        )
                      )
                  }
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
