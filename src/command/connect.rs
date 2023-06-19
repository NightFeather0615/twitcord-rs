use std::{sync::{Arc, OnceLock}, time::Duration};

use regex::Regex;
use rust_i18n::t;
use serenity::{
  model::prelude::{
    interaction::application_command::ApplicationCommandInteraction,
    PrivateChannel,
    Message
  },
  prelude::Context,
  builder::{
    CreateApplicationCommand,
    CreateMessage,
    CreateEmbed,
    CreateInteractionResponseFollowup,
    CreateEmbedFooter
  },
  utils::Color
};
use anyhow::Result;

use crate::core::{
  oauth::TwitterClient,
  utils::{
    EMBED_INFO_COLOR,
    EMBED_ERROR_COLOR,
    clean_up_dm,
    check_dm
  }
};


static AUTH_PIN_REGEX: OnceLock<Regex> = OnceLock::new();


pub async fn execute(
  context: &Context,
  interaction: &ApplicationCommandInteraction
) -> Result<()> {
  let is_dm: bool = check_dm(
    context,
    &interaction,
  ).await?;

  connect_account(
    context,
    &interaction,
    is_dm
  ).await
}

pub fn register(
  command: &mut CreateApplicationCommand
) -> &mut CreateApplicationCommand {
  command
    .name("connect")
    .name_localized("zh-TW", "連接")
    .name_localized("zh-CN", "连接")
    .description("Connect to your Twitter account")
    .description_localized("zh-TW", "連接你的 Twitter 帳號")
    .description_localized("zh-CN", "连接你的 Twitter 帐号")
}

fn build_auth_embed(
  embed: &mut CreateEmbed,
  auth_link: Arc<str>,
  locale: Arc<str>
) -> &mut CreateEmbed {
  embed
    .color(Color::new(EMBED_INFO_COLOR))
    .title(
      t!(
        "command.connect.auth-embed.title",
        locale = &locale
      )
    )
    .description(
      t!(
        "command.connect.auth-embed.description",
        locale = &locale,
        auth_link = auth_link
      )
    )
}

async fn connect_account(
  context: &Context,
  interaction: &ApplicationCommandInteraction,
  is_dm: bool
) -> Result<()> {
  let dm_channel: PrivateChannel = interaction
    .user
    .create_dm_channel(&context.http)
    .await?;

  clean_up_dm(context, &dm_channel).await?;

  let mut twitter_client: TwitterClient = TwitterClient::new(None, None)?;

  let auth_link: Arc<str> = twitter_client.get_authorization_url().await?;

  if is_dm {
    interaction.create_followup_message(
      &context.http,
      |response: &mut CreateInteractionResponseFollowup<'_>| {
        response
          .embed(
            |embed: &mut CreateEmbed| {
              build_auth_embed(
                embed,
                auth_link,
                interaction.locale.clone().into()
              )
            }
          )
      }
    ).await?;
  } else {
    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.add_embed(
        |embed: &mut CreateEmbed| {
          build_auth_embed(
            embed,
            auth_link,
            interaction.locale.clone().into()
          )
        }
      )
    ).await?;
  }

  let pin_code: Option<Arc<Message>> = interaction.user
    .await_reply(context)
    .author_id(interaction.user.id)
    .channel_id(dm_channel.id)
    .timeout(Duration::from_secs(75))
    .filter(
      |message: &Arc<Message>| {
        AUTH_PIN_REGEX
          .get_or_init(
            || {
              Regex::new(
                "[0-9]{7}"
              ).expect("Regex init failed.")
            }
          )
          .is_match(&message.content)
      }
    )
    .await;

  if pin_code.is_none() {
    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.add_embed(
        |embed: &mut CreateEmbed| {
          embed
            .color(Color::new(EMBED_ERROR_COLOR))
            .title(
              t!(
                "command.connect.timeout-embed.title",
                locale = &interaction.locale
              )
            )
            .description(
              t!(
                "command.connect.timeout-embed.description",
                locale = &interaction.locale
              )
            )
            .footer(
              |footer: &mut CreateEmbedFooter| {
                footer.text("ERR_TIMEOUT")
              }
            )
        }
      )
    ).await?;

    return Ok(());
  }

  let access_token_pair: Result<(&str, &str)> = twitter_client
    .get_access_token(&pin_code.unwrap().content)
    .await;

  if let Ok(access_token_pair) = access_token_pair {
    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.add_embed(
        |embed: &mut CreateEmbed| {
          embed
            .color(Color::new(EMBED_INFO_COLOR))
            .title(
              t!(
                "command.connect.success-embed.title",
                locale = &interaction.locale
              )
            )
            .description(
              t!(
                "command.connect.success-embed.description",
                locale = &interaction.locale
              )
            )
        }
      )
    ).await?;

    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.content(
        format!(
          "Twitter User Access Token\n||`{access_token}`||\n||`{access_token_secret}`||",
          access_token = access_token_pair.0,
          access_token_secret = access_token_pair.1
        )
      )
    ).await?.pin(&context.http).await?;

    return Ok(());
  } else {
    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.add_embed(
        |embed: &mut CreateEmbed| {
          embed
            .color(Color::new(EMBED_ERROR_COLOR))
            .title(
              t!(
                "command.connect.unauthorized-embed.title",
                locale = &interaction.locale
              )
            )
            .description(
              t!(
                "command.connect.unauthorized-embed.description",
                locale = &interaction.locale
              )
            )
            .footer(
              |footer: &mut CreateEmbedFooter| {
                footer.text("ERR_UNAUTHORIZED")
              }
            )
        }
      )
    ).await?;

    return Ok(());
  }
}
