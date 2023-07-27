# imitator-service

## Quickstart

```
# Build the docker image
docker build -t imitator-service .

# Run the image, mounting the messages.db file in the current working
# directory into the directory in the container where the app will look
# 
# This is ideal for development if you want to be able to easily inspect
# the contents of the database using tools on the hose
docker run --rm -p 8000:8000 \
    --mount type=bind,src=./messages.db,dst=/app-storage/messages.db \
    imitator-service
```