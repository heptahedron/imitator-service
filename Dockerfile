# adapted from https://blog.logrocket.com/packaging-a-rust-web-service-using-docker/
FROM rust:1.71.0-slim-buster as builder

RUN USER=root cargo new --bin imitator-service
WORKDIR /imitator-service
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/imitator_service*
RUN cargo build --release


FROM debian:buster-slim
ARG APP_DIR=/usr/local/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 8000

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP_DIR}

COPY --from=builder /imitator-service/target/release/imitator-service ${APP_DIR}/imitator-service

RUN mkdir /app-storage

RUN chown -R $APP_USER:$APP_USER ${APP_DIR} /app-storage

VOLUME ["/app-storage"]

USER $APP_USER
WORKDIR ${APP_DIR}

CMD ["./imitator-service", "--db=/app-storage/messages.db", "serve", "0.0.0.0:8000"]