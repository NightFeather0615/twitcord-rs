use std::{env, sync::{Arc, OnceLock}, time::Duration};

use regex::Regex;
use serenity::{
  model::prelude::{
    interaction::{
      application_command::ApplicationCommandInteraction,
      InteractionResponseType
    },
    PrivateChannel,
    Message
  },
  prelude::Context,
  builder::{
    CreateApplicationCommand,
    CreateMessage,
    CreateEmbed
  },
  utils::Color
};
use anyhow::Result;

use crate::{
  oauth::TwitterClient,
  utils::{
    EMBED_INFO_COLOR,
    EMBED_ERROR_COLOR,
    TWITTER_CONSUMER_KEY,
    TWITTER_CONSUMER_SECRET,
    clean_up_dm
  }
};


static AUTH_PIN_REGEX: OnceLock<Regex> = OnceLock::new();


pub async fn execute(
  context: &Context,
  interaction: &ApplicationCommandInteraction
) -> Result<()> {
  let is_dm: bool = context.http.get_channel(
    *interaction.channel_id.as_u64()
  ).await?.private().is_some();

  if is_dm {
    interaction.defer(&context.http).await?;
  } else {
    interaction.create_interaction_response(
      &context.http,
      |response| {
        response
          .kind(
            InteractionResponseType::ChannelMessageWithSource
          )
          .interaction_response_data(
            |message| {
              message
                .ephemeral(true)
                .content(
                  "Please check your DM, and make sure you turned allow direct messages on"
                )
            }
          )
      }
    ).await?;
  }

  connect_account(
    context,
    &interaction, is_dm
  ).await
}

pub fn register(
  command: &mut CreateApplicationCommand
) -> &mut CreateApplicationCommand {
  command
    .name("connect")
    .description("Connect to your Twitter account")
    .name_localized("zh-TW", "連接")
    .description_localized("zh-TW", "連接你的 Twitter 帳號")
    .name_localized("zh-CN", "连接")
    .description_localized("zh-CN", "连接你的 Twitter 帐号")
}

fn build_auth_embed(embed: &mut CreateEmbed, auth_link: Arc<str>) -> &mut CreateEmbed {
  embed
    .color(Color::new(EMBED_INFO_COLOR))
    .title(":link: Connect to Your Twitter Account")
    .description(
      format!(
        "Please go to [Twitter API Authorize]({auth_link}), click on \"Authorize app\", then send the verification PIN code here within 60 seconds",
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

  let mut twitter_client: TwitterClient = TwitterClient::new(
    TWITTER_CONSUMER_KEY.get_or_init(
      || {
        env::var("TWITTER_CONSUMER_KEY")
          .expect("TWITTER_CONSUMER_KEY is not set.")
          .into()
      }
    ).clone(),
    TWITTER_CONSUMER_SECRET.get_or_init(
      || {
        env::var("TWITTER_CONSUMER_SECRET")
          .expect("TWITTER_CONSUMER_SECRET is not set.")
          .into()
      }
    ).clone(),
    None,
    None
  )?;

  let auth_link: Arc<str> = twitter_client.get_authorization_url().await?;

  if is_dm {
    interaction.create_followup_message(
      &context.http,
      |response| {
        response
          .embed(
            |embed: &mut CreateEmbed| {
              build_auth_embed(embed, auth_link)
            }
          )
      }
    ).await?;
  } else {
    dm_channel.send_message(
      &context.http,
      |message: &mut CreateMessage<'_>| message.add_embed(
        |embed: &mut CreateEmbed| {
          build_auth_embed(embed, auth_link)
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
            || Regex::new(
              "[0-9]{7}"
            ).expect("Regex init failed.")
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
            .title(":warning: Connect Failed")
            .description("Authorization timeout, please try again")
            .footer(
              |footer| footer.text("ERR_TIMEOUT")
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
            .title(":white_check_mark: Account Connected")
            .description("You can disconnect to your account by using `/disconnect` at any time")
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
            .title(":warning: Connect Failed")
            .description("Unauthorized PIN code")
            .footer(
              |footer| footer.text("ERR_UNAUTHORIZED")
            )
        }
      )
    ).await?;

    return Ok(());
  }
}
