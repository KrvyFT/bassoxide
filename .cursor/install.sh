#!/usr/bin/env bash
# Bassoxide Cloud Agent 环境安装脚本。
# 幂等：可重复运行；准备构建/运行 GUI + 音频解码所需的系统依赖与 Rust 工具链。
set -euo pipefail

echo "==> Installing system build & runtime dependencies"
export DEBIAN_FRONTEND=noninteractive
sudo apt-get update -qq
sudo apt-get install -y --no-install-recommends \
  pkg-config \
  libasound2-dev \
  libgl1-mesa-dev \
  libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libxkbcommon-x11-dev \
  libwayland-dev \
  libudev-dev \
  fonts-noto-cjk \
  xvfb \
  curl \
  unzip \
  ffmpeg

# Guitar Pro 依赖 (rfd / eframe / cpal) 拉取的 crate 需要 edition2024，
# 需要 Rust >= 1.85，而基础镜像自带 1.83，这里升级到最新 stable。
echo "==> Ensuring Rust stable toolchain (>= 1.85 for edition2024)"
rustup toolchain install stable --profile minimal --no-self-update
rustup default stable

echo "==> Building workspace"
cargo build --workspace

echo "==> Install complete"
