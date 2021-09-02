# x11docker/umbrewm
# 
# Run umberwm desktop in docker. 
# Use x11docker to run image. 
# Get x11docker from github: 
#   https://github.com/mviereck/x11docker 
#
# Examples: 
#   - Run desktop:
#       x11docker --desktop x11docker/umberwm
#   - Run single application:
#       x11docker x11docker/umberwm thunar
#
# Options:
# Persistent home folder stored on host with   --home
# Shared host folder with                      --share DIR
# Hardware acceleration with option            --gpu
# Clipboard sharing with option                --clipboard
# ALSA sound support with option               --alsa
# Pulseaudio sound support with option         --pulseaudio
# Language setting with                        --lang [=$LANG]
# Printing over CUPS with                      --printer
# Webcam support with                          --webcam
#
# Look at x11docker --help for further options.

FROM rust:1-buster

RUN apt-get update && \
    env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      libxcb-randr0-dev \
      libfreetype6-dev \
      libfontconfig1-dev \
      libxcb-xfixes0-dev \
      libxkbcommon-dev \
      rofi \
      libxcursor-dev \
      libxrandr-dev \
      libxi-dev \
      libgl1-mesa-glx \
      libgl-dev \
      compton \
      x11-apps \
      x11-utils \
      hsetroot && \
      cargo install umberwm alacritty && \
      useradd -m user --uid=1000
RUN chsh -s /bin/bash user
USER user
WORKDIR /home/user
RUN mkdir ~/.config
COPY umberwm.ron .config/umberwm.ron
COPY action-handler.sh .
COPY alacritty-umbertutor.yml .
COPY umbertutor.sh .
COPY ./start-umberwm .
CMD ["./start-umberwm"]
