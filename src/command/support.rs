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
                      "[GitHub Issues](https://github.com/NightFeather0615/twitcord-rs/issues)"
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
    .name("support")
    .name_localized("zh-TW", "支援")
    .name_localized("zh-CN", "支援")
    .description("Contact developers outside Discord")
    .description_localized("zh-TW", "在 Discord 外與開發者聯繫")
    .description_localized("zh-CN", "在 Discord 外与开发者联系")
}
