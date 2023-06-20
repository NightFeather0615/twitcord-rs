FROM rust:1.70.0-slim-buster AS builder

RUN update-ca-certificates

WORKDIR /twitcord-rs

COPY ./ .

RUN cargo build --release


FROM gcr.io/distroless/cc

WORKDIR /twitcord-rs

ENV DISCORD_BOT_TOKEN your_deploy_token
ENV TWITTER_CONSUMER_KEY your_deploy_key
ENV TWITTER_CONSUMER_SECRET your_deploy_secret

COPY --from=builder /twitcord-rs/target/release/twitcord-rs ./

CMD ["/twitcord-rs/twitcord-rs"]