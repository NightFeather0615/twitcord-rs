use rust_i18n::t;
use serenity::{
  model::prelude::interaction::{
    application_command::ApplicationCommandInteraction,
    InteractionResponseType
  },
  prelude::Context,
  builder::{
    CreateApplicationCommand,
    CreateInteractionResponse,
    CreateInteractionResponseData
  },
  utils::Color
};
use anyhow::Result;

use crate::core::utils::EMBED_INFO_COLOR;


pub async fn execute(
  context: &Context,
  interaction: &ApplicationCommandInteraction
) -> Result<()> {
  interaction.create_interaction_response(
    &context.http,
    |response: &mut CreateInteractionResponse<'_>| {
      response
        .kind(InteractionResponseType::ChannelMessageWithSource)
        .interaction_response_data(
          |message: &mut CreateInteractionResponseData<'_>| {
            message
              .ephemeral(true)
              .embed(
                |embed| {
                  embed
                    .color(Color::new(EMBED_INFO_COLOR))
                    .description(
                      t!(
                        "command.invite.info-embed.description",
                        locale = &interaction.locale,
                        client_id = interaction.application_id.as_u64()
                      )
                    )
                }
              )
          }
        )
    }
  ).await?;

  Ok(())
}

pub fn register(
  command: &mut CreateApplicationCommand
) -> &mut CreateApplicationCommand {
  command
    .name("invite")
    .name_localized("zh-TW", "邀請連結")
    .name_localized("zh-CN", "邀请连结")
    .description("Show invite link")
    .description_localized("zh-TW", "顯示機器人邀請連結")
    .description_localized("zh-CN", "显示机器人邀请连结")
}
