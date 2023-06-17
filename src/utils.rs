use std::sync::{OnceLock, Arc};

use serenity::{
  model::prelude::PrivateChannel,
  prelude::Context,
  builder::{
    GetMessages,
    EditMessage
  },
  Error,
};


pub static TWITTER_CONSUMER_KEY: OnceLock<Arc<str>> = OnceLock::new();
pub static TWITTER_CONSUMER_SECRET: OnceLock<Arc<str>> = OnceLock::new();


pub static EMBED_INFO_COLOR: u32 = 0x3983f2;
pub static EMBED_ERROR_COLOR: u32 = 0xeca42c;


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
