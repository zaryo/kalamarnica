FROM rust:1.94

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    g++ \
    clang \
    libssl-dev \
    pkg-config

WORKDIR /app

RUN git clone https://github.com/zaryo/kalamarnica.git .

RUN cargo build

RUN mv target/debug/kalamarnica /usr/local/bin/kalamarnica

ENTRYPOINT ["kalamarnica"]
