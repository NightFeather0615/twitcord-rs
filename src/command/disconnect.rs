use std::sync::Arc;

use rust_i18n::t;
use serenity::{
  model::prelude::{
    interaction::application_command::ApplicationCommandInteraction,
    PrivateChannel
  },
  prelude::Context,
  builder::{
    CreateApplicationCommand,
    CreateMessage,
    CreateEmbed,
    CreateInteractionResponseFollowup
  },
  utils::Color
};
use anyhow::Result;

use crate::core::{
  utils::{
    EMBED_INFO_COLOR,
    clean_up_dm,
    check_dm
  },
  cache::AccessTokenCache
};


pub async fn execute(
  context: &Context,
  interaction: &ApplicationCommandInteraction
) -> Result<()> {
  let is_dm: bool = check_dm(
    context,
    &interaction,
  ).await?;

  disconnect_account(
    context,
    &interaction,
    is_dm
  ).await
}

pub fn register(
  command: &mut CreateApplicationCommand
) -> &mut CreateApplicationCommand {
  command
    .name("disconnect")
    .name_localized("zh-TW", "斷開連接")
    .name_localized("zh-CN", "断开连接")
    .description("Disconnect to your Twitter account")
    .description_localized("zh-TW", "中斷與 Twitter 帳號的連接")
    .description_localized("zh-CN", "中断与 Twitter 帐号的连接")
}

fn build_success_embed(
  embed: &mut CreateEmbed,
  locale: Arc<str>
) -> &mut CreateEmbed {
  embed
    .color(Color::new(EMBED_INFO_COLOR))
    .title(
      t!(
        "command.disconnect.success-embed.title",
        locale = &locale
      )
    )
    .description(
      t!(
        "command.disconnect.success-embed.description",
        locale = &locale
      )
    )
}

async fn disconnect_account(
  context: &Context,
  interaction: &ApplicationCommandInteraction,
  is_dm: bool
) -> Result<()> {
  let dm_channel: PrivateChannel = interaction
    .user
    .create_dm_channel(&context.http)
    .await?;

  clean_up_dm(context, &dm_channel).await?;

  AccessTokenCache::get().purge(
    *interaction.user.id.as_u64()
  ).await;

  if is_dm {
    interaction.create_followup_message(
      &context.http,
      |response: &mut CreateInteractionResponseFollowup<'_>| {
        response
          .embed(
            |embed: &mut CreateEmbed| {
              build_success_embed(
                embed,
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
          build_success_embed(
            embed,
            interaction.locale.clone().into()
          )
        }
      )
    ).await?;
  }

  Ok(())
}
