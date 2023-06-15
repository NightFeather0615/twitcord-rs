import tweepy, logging

logging.basicConfig(
  level = logging.DEBUG,
  format = '[%(asctime)s] [%(levelname)s] %(message)s',
  datefmt = '%Y/%m/%d %I:%M:%S'
)


TWITTER_CONSUMER_KEY = "***"
TWITTER_CONSUMER_SECRET = "***"
TWITTER_BEARER_TOKEN = "***"
auth = tweepy.OAuthHandler(TWITTER_CONSUMER_KEY, TWITTER_CONSUMER_SECRET)
print(auth.get_authorization_url())