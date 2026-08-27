#!/usr/bin/env bash
# Bassoxide Cloud Agent 环境安装脚本。
# 幂等：可重复运行；准备构建/运行 GUI + 音频所需的系统依赖与 Rust 工具链。
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
  fluid-soundfont-gm

# Guitar Pro 依赖 (rfd / eframe / cpal) 拉取的 crate 需要 edition2024，
# 需要 Rust >= 1.85，而基础镜像自带 1.83，这里升级到最新 stable。
echo "==> Ensuring Rust stable toolchain (>= 1.85 for edition2024)"
rustup toolchain install stable --profile minimal --no-self-update
rustup default stable

# 内置乐队音源：GeneralUser GS → assets/Bassoxide_Band.sf2
# 覆盖电吉他 / 电贝斯 / 键盘 / 鼓；失败时回退到系统 FluidR3_GM
echo "==> Ensuring band SoundFont (assets/Bassoxide_Band.sf2)"
SF2_DEST="assets/Bassoxide_Band.sf2"
mkdir -p assets
if [ -f "$SF2_DEST" ] && [ "$(stat -c%s "$SF2_DEST" 2>/dev/null || echo 0)" -gt 1000000 ]; then
  echo "    SoundFont already present: $SF2_DEST"
else
  TMP="$(mktemp -d)"
  OK=0
  URLS=(
    "https://www.dropbox.com/s/4x27l49kxcwamp5/GeneralUser_GS_1.471.zip?dl=1"
  )
  for url in "${URLS[@]}"; do
    echo "    Trying $url"
    if curl -fL --connect-timeout 30 --max-time 300 -o "$TMP/dl" "$url"; then
      if head -c 4 "$TMP/dl" | grep -q RIFF; then
        cp "$TMP/dl" "$SF2_DEST"
        OK=1
        break
      fi
      if unzip -l "$TMP/dl" >/dev/null 2>&1; then
        unzip -o "$TMP/dl" -d "$TMP/out" >/dev/null
        FOUND="$(find "$TMP/out" -iname '*.sf2' | head -1 || true)"
        if [ -n "$FOUND" ]; then
          cp "$FOUND" "$SF2_DEST"
          OK=1
          break
        fi
      fi
    fi
  done
  rm -rf "$TMP"
  if [ "$OK" != 1 ]; then
    echo "    GeneralUser download failed; copying FluidR3_GM as fallback"
    if [ -f /usr/share/sounds/sf2/FluidR3_GM.sf2 ]; then
      cp /usr/share/sounds/sf2/FluidR3_GM.sf2 "$SF2_DEST"
    else
      echo "    ERROR: no SoundFont available"
      exit 1
    fi
  fi
  echo "    Installed $(stat -c%s "$SF2_DEST") bytes → $SF2_DEST"
fi

echo "==> Building workspace"
cargo build --workspace

echo "==> Install complete"
