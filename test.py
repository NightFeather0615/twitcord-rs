import tweepy, logging, os
from dotenv import load_dotenv

logging.basicConfig(
  level = logging.DEBUG,
  format = '[%(asctime)s] [%(levelname)s] %(message)s',
  datefmt = '%Y/%m/%d %I:%M:%S'
)



load_dotenv()
TWITTER_CONSUMER_KEY = os.getenv("TWITTER_CONSUMER_KEY")
TWITTER_CONSUMER_SECRET = os.getenv("TWITTER_CONSUMER_SECRET")
TWITTER_BEARER_TOKEN = os.getenv("TWITTER_BEARER_TOKEN")
auth = tweepy.OAuthHandler(TWITTER_CONSUMER_KEY, TWITTER_CONSUMER_SECRET)
print(auth.get_authorization_url())
auth.get_access_token(input())
user = tweepy.Client(
  bearer_token = TWITTER_BEARER_TOKEN,
  consumer_key = TWITTER_CONSUMER_KEY,
  consumer_secret = TWITTER_CONSUMER_SECRET,
  access_token = auth.access_token,
  access_token_secret = auth.access_token_secret
)
user.like("1669464988161908738")