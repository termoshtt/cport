FROM debian:buster
LABEL maintainer "Toshiki Teramura <toshiki.teramura@gmail.com>"

RUN apt-get update     \
 && apt-get install -y \
    cmake              \
    g++                \
    libboost-all-dev   \
    make               \
 && apt-get clean      \
 && rm -rf /var/lib/apt/lists/*

