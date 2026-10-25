FROM public.ecr.aws/docker/library/rust:trixie

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libpq-dev \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
        git \
    && rm -rf /var/lib/apt/lists/*
